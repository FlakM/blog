use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::collections::HashSet;
use url::Url;

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
    pub reply_items: Vec<DiscussionReply>,
}

#[derive(FromRow)]
struct InteractionCounts {
    boosts: i64,
}

#[derive(FromRow)]
struct ReplyRow {
    object_id: String,
    in_reply_to: Option<String>,
    actor_id: String,
    username: String,
    display_name: Option<String>,
    profile_url: Option<String>,
    avatar_url: Option<String>,
    url: String,
    content: String,
    published_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct DiscussionReply {
    pub id: String,
    pub in_reply_to: Option<String>,
    pub author: String,
    pub author_name: String,
    pub author_url: String,
    pub avatar_url: Option<String>,
    pub url: String,
    pub content: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
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
        "SELECT COUNT(*) AS boosts FROM fediverse_reactions WHERE post_slug = $1 AND kind = 'Announce'",
    )
    .bind(&post_slug)
    .fetch_one(&pool);
    let replies = sqlx::query_as::<_, ReplyRow>(
        "SELECT r.object_id, r.in_reply_to, r.actor_id, a.username, a.display_name, a.profile_url, a.avatar_url, r.url, r.content, r.published_at, r.updated_at FROM fediverse_replies r JOIN fediverse_actors a ON a.ap_id = r.actor_id WHERE r.post_slug = $1 AND r.deleted_at IS NULL ORDER BY r.published_at, r.object_id",
    )
    .bind(&post_slug)
    .fetch_all(&pool);
    let (links, counts, replies) = tokio::try_join!(links, counts, replies).map_err(|error| {
        tracing::warn!(%error, "Failed to retrieve discussion links");
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let reply_items = replies.into_iter().map(sanitize_reply).collect::<Vec<_>>();

    Ok(Json(DiscussionLinksResponse {
        links,
        replies: reply_items.len() as i64,
        boosts: counts.boosts,
        reply_items,
    }))
}

fn sanitize_reply(reply: ReplyRow) -> DiscussionReply {
    let actor = Url::parse(&reply.actor_id).ok();
    let author = actor
        .as_ref()
        .and_then(Url::host_str)
        .map(|host| format!("@{}@{host}", reply.username))
        .unwrap_or_else(|| format!("@{}", reply.username));
    let url = Url::parse(&reply.url)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map_or_else(|| reply.object_id.clone(), |url| url.to_string());
    let author_url = reply
        .profile_url
        .as_deref()
        .and_then(|url| Url::parse(url).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map_or(reply.actor_id, |url| url.to_string());
    let avatar_url = reply
        .avatar_url
        .as_deref()
        .and_then(|url| Url::parse(url).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(|url| url.to_string());
    DiscussionReply {
        id: reply.object_id,
        in_reply_to: reply.in_reply_to,
        author,
        author_name: reply
            .display_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(reply.username),
        author_url,
        avatar_url,
        url,
        content: sanitize_html(&reply.content),
        published_at: reply.published_at,
        updated_at: reply.updated_at,
    }
}

fn sanitize_html(content: &str) -> String {
    ammonia::Builder::default()
        .tags(HashSet::from([
            "a",
            "b",
            "blockquote",
            "br",
            "code",
            "del",
            "em",
            "i",
            "li",
            "ol",
            "p",
            "pre",
            "span",
            "strong",
            "u",
            "ul",
        ]))
        .url_schemes(HashSet::from(["http", "https"]))
        .link_rel(Some("nofollow ugc noopener noreferrer"))
        .clean(content)
        .to_string()
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
            reply_items: vec![],
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
                "boosts": 1,
                "reply_items": []
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
            "boosts": 0,
            "reply_items": []
        });

        let response: DiscussionLinksResponse = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(response.links.len(), 2);
        assert_eq!(serde_json::to_value(response).unwrap(), json);
    }

    #[test]
    fn remote_reply_html_is_sanitized() {
        let clean = sanitize_html(
            r#"<p>Hello <strong>world</strong><script>alert(1)</script><a href="javascript:alert(1)" onclick="alert(2)">bad</a><a href="https://example.com">safe</a></p>"#,
        );

        assert!(clean.contains("<strong>world</strong>"));
        assert!(clean.contains("https://example.com"));
        assert!(clean.contains("nofollow ugc noopener noreferrer"));
        assert!(!clean.contains("<script"));
        assert!(!clean.contains("javascript:"));
        assert!(!clean.contains("onclick"));
    }
}
