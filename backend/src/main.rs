use crate::hugo_posts::HugoBlogPost;
use error::Error;
use sqlx::PgPool;
use std::net::SocketAddr;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, instrument};

mod correlation;
mod database;
mod error;
mod hugo_posts;
mod likes;
mod observability;

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Error> {
    // Initialize observability stack first (tracing, metrics, logging)
    let prometheus_handle =
        observability::init_observability().expect("Failed to initialize observability");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://blog:blog@localhost:5432/blog".to_string());

    info!(
        "Connecting to PostgreSQL database: {}",
        database_url.replace(&extract_password(&database_url), "***")
    );

    let pool = PgPool::connect(&database_url).await.map_err(|e| {
        tracing::error!("Failed to connect to PostgreSQL: {}", e);
        e
    })?;

    // Run migrations in ./migrations
    sqlx::migrate!().run(&pool).await?;
    info!("Migrations run");

    let posts_path = std::env::args().nth(1).expect("No posts file given");
    let blog_repo = hugo_posts::BlogRepository { db: pool.clone() };
    let blog_posts = HugoBlogPost::load_new_posts(posts_path).expect("Failed to load blog posts");

    for blog_post in blog_posts {
        info!("Processing: {}", blog_post.slug);
        blog_repo.new_blog_entry(&blog_post).await?;
    }

    info!("Blog posts processed successfully");

    // Create the Axum app with routes and middleware
    let app = Router::new()
        .route("/like/:post_slug", post(likes::like_post))
        .route("/likes/:post_slug", get(likes::get_likes))
        .route("/health", get(health_check))
        .layer(middleware::from_fn(correlation::correlation_middleware))
        .layer(CorsLayer::permissive()) // Allow CORS for frontend
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<axum::body::Body>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        correlation_id = tracing::field::Empty,
                        request_id = tracing::field::Empty,
                        session_id = tracing::field::Empty,
                        user_id = tracing::field::Empty,
                        user_agent = tracing::field::Empty,
                        remote_ip = tracing::field::Empty,
                        forwarded_for = tracing::field::Empty,
                        status_code = tracing::field::Empty,
                        latency_ms = tracing::field::Empty,
                    )
                })
                .on_response(
                    |response: &axum::response::Response,
                     latency: std::time::Duration,
                     span: &tracing::Span| {
                        span.record("status_code", response.status().as_u16());
                        span.record("latency_ms", latency.as_millis() as f64);
                    },
                ),
        )
        .with_state(pool);

    // Create separate metrics server without any tracing instrumentation
    let metrics_app = prometheus_handle
        .clone()
        .map(|handle| Router::new().route("/metrics", get(move || async move { handle.render() })));

    // Start the web server
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let addr: SocketAddr = bind_address.parse().expect("Invalid bind address");

    info!("Starting web server on {}", addr);

    // Setup graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    // Start metrics server if available
    let metrics_server = if let Some(metrics_app) = metrics_app {
        let metrics_addr: SocketAddr = "127.0.0.1:9090".parse().expect("Invalid metrics address");
        let metrics_listener = tokio::net::TcpListener::bind(metrics_addr).await?;
        info!("Starting metrics server on {}", metrics_addr);
        Some(axum::serve(metrics_listener, metrics_app).with_graceful_shutdown(shutdown_signal()))
    } else {
        None
    };

    // Run both servers concurrently
    if let Some(metrics_server) = metrics_server {
        if let Err(e) = tokio::try_join!(server, metrics_server) {
            tracing::error!("Server error: {}", e);
        }
    } else if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    // Shutdown observability providers
    observability::shutdown_observability();

    Ok(())
}

#[instrument]
async fn health_check() -> &'static str {
    "OK"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}

fn extract_password(database_url: &str) -> String {
    if let Some(start) = database_url.find("://") {
        if let Some(end) = database_url[start + 3..].find('@') {
            let auth_part = &database_url[start + 3..start + 3 + end];
            if let Some(colon_pos) = auth_part.find(':') {
                return auth_part[colon_pos + 1..].to_string();
            }
        }
    }
    "".to_string()
}
