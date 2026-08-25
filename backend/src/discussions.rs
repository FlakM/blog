use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Deserialize, FromRow, PartialEq, Serialize)]
pub struct DiscussionLink {
    pub source: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DiscussionLinksResponse {
    pub links: Vec<DiscussionLink>,
    pub replies: i64,
    pub boosts: i64,
}

#[derive(FromRow)]
struct InteractionCounts {
    replies: i64,
    boosts: i64,
}

pub async fn get_discussion_links(
    Path(post_slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<Json<DiscussionLinksResponse>, axum::http::StatusCode> {
    let links = sqlx::query_as::<_, DiscussionLink>(
        "SELECT source, label, url FROM blog_post_discussion_links WHERE post_slug = $1 ORDER BY created_at, source, url",
    )
    .bind(&post_slug)
    .fetch_all(&pool);
    let counts = sqlx::query_as::<_, InteractionCounts>(
        "SELECT (SELECT COUNT(*) FROM fediverse_replies WHERE post_slug = $1 AND deleted_at IS NULL) AS replies, (SELECT COUNT(*) FROM fediverse_reactions WHERE post_slug = $1 AND kind = 'Announce') AS boosts",
    )
    .bind(&post_slug)
    .fetch_one(&pool);
    let (links, counts) = tokio::try_join!(links, counts).map_err(|error| {
        tracing::warn!(%error, "Failed to retrieve discussion links");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DiscussionLinksResponse {
        links,
        replies: counts.replies,
        boosts: counts.boosts,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_has_stable_extensible_shape() {
        let response = DiscussionLinksResponse {
            links: vec![DiscussionLink {
                source: "hacker_news".to_string(),
                label: "Hacker News".to_string(),
                url: "https://news.ycombinator.com/item?id=1".to_string(),
            }],
            replies: 2,
            boosts: 1,
        };

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({
                "links": [{
                    "source": "hacker_news",
                    "label": "Hacker News",
                    "url": "https://news.ycombinator.com/item?id=1"
                }],
                "replies": 2,
                "boosts": 1
            })
        );
    }

    #[test]
    fn response_round_trips_multiple_sources() {
        let json = serde_json::json!({
            "links": [
                {
                    "source": "mastodon",
                    "label": "Mastodon",
                    "url": "https://fedi.example/blog/posts/example"
                },
                {
                    "source": "reddit",
                    "label": "Reddit",
                    "url": "https://www.reddit.com/r/rust/comments/example"
                }
            ],
            "replies": 0,
            "boosts": 0
        });

        let response: DiscussionLinksResponse = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(response.links.len(), 2);
        assert_eq!(serde_json::to_value(response).unwrap(), json);
    }
}
