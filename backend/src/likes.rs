use crate::correlation::CorrelationContext;
use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use chrono::{DateTime, Utc};
use metrics::{counter, gauge, histogram};
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::PgPool;
use tracing::{info, instrument, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct LikeResponse {
    pub success: bool,
    pub message: String,
    pub total_likes: i64,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct LikeRecord {
    post_slug: String,
    user_ip: String,
    liked_at: DateTime<Utc>,
}

#[instrument(skip(pool, headers, correlation_ctx), fields(post_slug = %post_slug))]
pub async fn like_post(
    Path(post_slug): Path<String>,
    State(pool): State<PgPool>,
    Extension(correlation_ctx): Extension<CorrelationContext>,
    headers: HeaderMap,
) -> Result<Json<LikeResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    counter!("blog_likes_requests_total", "endpoint" => "like_post").increment(1);

    // Extract IP address from Cloudflare headers or fallback
    let user_ip = extract_user_ip(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let cf_country = headers
        .get("cf-ipcountry")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let cf_connecting_ip = headers
        .get("cf-connecting-ip")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    info!(
        post_slug = %post_slug,
        user_ip = %user_ip,
        user_agent = %user_agent,
        cf_country = ?cf_country,
        correlation_id = %correlation_ctx.correlation_id,
        request_id = %correlation_ctx.request_id,
        "Processing like request"
    );

    // Check if the post exists
    let post_exists = sqlx::query!("SELECT slug FROM blog_posts WHERE slug = $1", post_slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            warn!("Database error checking post existence: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if post_exists.is_none() {
        counter!("blog_likes_errors_total", "reason" => "post_not_found").increment(1);
        histogram!("blog_likes_request_duration_ms", "endpoint" => "like_post", "status" => "error")
            .record(start_time.elapsed().as_millis() as f64);
        return Ok(Json(LikeResponse {
            success: false,
            message: "Blog post not found".to_string(),
            total_likes: 0,
        }));
    }

    // Create hour bucket for rate limiting (format: 'YYYY-MM-DD HH')
    let now = Utc::now();
    let hour_bucket = now.format("%Y-%m-%d %H").to_string();

    // Parse and validate IP addresses
    let validated_user_ip = user_ip
        .parse::<IpNetwork>()
        .unwrap_or_else(|_| "127.0.0.1".parse().unwrap());

    let validated_cf_ip = cf_connecting_ip
        .as_ref()
        .and_then(|ip| ip.parse::<IpNetwork>().ok());

    let result = sqlx::query!(
        r#"
        INSERT INTO blog_post_likes (post_slug, user_ip, user_agent, cf_country, cf_connecting_ip, liked_at, hour_bucket)
        VALUES ($1, $2, $3, $4, $5, NOW(), $6)
        "#,
        post_slug,
        validated_user_ip,
        user_agent,
        cf_country,
        validated_cf_ip,
        hour_bucket
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            info!(post_slug = %post_slug, user_ip = %user_ip, "Like recorded successfully");
            counter!("blog_likes_successful_total").increment(1);
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            info!(post_slug = %post_slug, user_ip = %user_ip, "Like already exists within rate limit window");
            counter!("blog_likes_rate_limited_total").increment(1);
            histogram!("blog_likes_request_duration_ms", "endpoint" => "like_post", "status" => "rate_limited")
                .record(start_time.elapsed().as_millis() as f64);
            return Ok(Json(LikeResponse {
                success: false,
                message: "You can only like a post once per hour".to_string(),
                total_likes: get_like_count(&pool, &post_slug).await.unwrap_or(0),
            }));
        }
        Err(e) => {
            warn!("Database error inserting like: {}", e);
            counter!("blog_likes_errors_total", "reason" => "database_error").increment(1);
            histogram!("blog_likes_request_duration_ms", "endpoint" => "like_post", "status" => "error")
                .record(start_time.elapsed().as_millis() as f64);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Get the total like count for this post
    let total_likes = get_like_count(&pool, &post_slug).await.unwrap_or(0);

    // Update metrics
    gauge!("blog_post_likes_total", "post_slug" => post_slug.clone()).set(total_likes as f64);
    histogram!("blog_likes_request_duration_ms", "endpoint" => "like_post", "status" => "success")
        .record(start_time.elapsed().as_millis() as f64);

    Ok(Json(LikeResponse {
        success: true,
        message: "Like recorded successfully".to_string(),
        total_likes,
    }))
}

#[instrument(skip(pool, correlation_ctx), fields(post_slug = %post_slug))]
pub async fn get_likes(
    Path(post_slug): Path<String>,
    State(pool): State<PgPool>,
    Extension(correlation_ctx): Extension<CorrelationContext>,
) -> Result<Json<LikeResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    counter!("blog_likes_requests_total", "endpoint" => "get_likes").increment(1);

    info!(
        post_slug = %post_slug,
        correlation_id = %correlation_ctx.correlation_id,
        request_id = %correlation_ctx.request_id,
        "Retrieving like count"
    );

    let total_likes = get_like_count(&pool, &post_slug).await.map_err(|e| {
        warn!(
            error = %e,
            post_slug = %post_slug,
            correlation_id = %correlation_ctx.correlation_id,
            "Database error getting like count"
        );
        counter!("blog_likes_errors_total", "reason" => "database_error").increment(1);
        histogram!("blog_likes_request_duration_ms", "endpoint" => "get_likes", "status" => "error")
            .record(start_time.elapsed().as_millis() as f64);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(
        post_slug = %post_slug,
        total_likes = %total_likes,
        correlation_id = %correlation_ctx.correlation_id,
        "Like count retrieved successfully"
    );

    histogram!("blog_likes_request_duration_ms", "endpoint" => "get_likes", "status" => "success")
        .record(start_time.elapsed().as_millis() as f64);

    Ok(Json(LikeResponse {
        success: true,
        message: "Like count retrieved successfully".to_string(),
        total_likes,
    }))
}

#[instrument(skip(pool))]
async fn get_like_count(pool: &PgPool, post_slug: &str) -> Result<i64, sqlx::Error> {
    let start_time = std::time::Instant::now();

    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM blog_post_likes WHERE post_slug = $1",
        post_slug
    )
    .fetch_one(pool)
    .await?;

    let count: i64 = result.count.unwrap_or(0);

    histogram!("blog_database_query_duration_ms", "query" => "get_like_count")
        .record(start_time.elapsed().as_millis() as f64);

    Ok(count)
}

fn extract_user_ip(headers: &HeaderMap) -> String {
    // Try to get the real IP from Cloudflare headers first
    if let Some(cf_connecting_ip) = headers.get("cf-connecting-ip") {
        if let Ok(ip) = cf_connecting_ip.to_str() {
            return ip.to_string();
        }
    }

    // Fallback to other common headers
    let ip_headers = [
        "x-forwarded-for",
        "x-real-ip",
        "x-client-ip",
        "cf-pseudo-ipv4",
    ];

    for header_name in &ip_headers {
        if let Some(header_value) = headers.get(*header_name) {
            if let Ok(ip_str) = header_value.to_str() {
                // X-Forwarded-For can contain multiple IPs, take the first one
                let ip = ip_str.split(',').next().unwrap_or(ip_str).trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    // Ultimate fallback
    "unknown".to_string()
}
