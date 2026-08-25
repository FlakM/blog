use crate::{
    error::Error,
    hugo_posts::{BlogRepository, HugoBlogPost},
};
use activitypub_federation::{
    activity_sending::SendActivityTask,
    axum::{
        inbox::{receive_activity, ActivityData},
        json::FederationJson,
    },
    config::{Data, FederationConfig, FederationMiddleware},
    fetch::{
        object_id::ObjectId,
        webfinger::{build_webfinger_response, extract_webfinger_name, Webfinger},
    },
    http_signatures::generate_actor_keypair,
    protocol::{
        context::WithContext, helpers::deserialize_one_or_many, public_key::PublicKey,
        verification::verify_domains_match,
    },
    traits::{Activity, Actor, Object},
};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::{collections::HashSet, fmt::Debug, fs, time::Duration};
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
pub struct FediverseRepository {
    pool: PgPool,
    domain: String,
    username: String,
    blog_domain: String,
}

#[derive(Clone)]
pub struct MastodonResolver {
    client: reqwest::Client,
    instance: Url,
    token: String,
}

#[derive(Debug, Deserialize)]
struct MastodonSearch {
    statuses: Vec<MastodonStatus>,
}

#[derive(Debug, Deserialize)]
struct MastodonStatus {
    uri: Url,
}

impl MastodonResolver {
    pub fn from_env() -> Option<Self> {
        let instance = std::env::var("PREFERRED_MASTODON_INSTANCE").ok()?;
        let token_file = match std::env::var("MASTODON_ACCESS_TOKEN_FILE") {
            Ok(path) => path,
            Err(_) => {
                tracing::warn!("Preferred Mastodon instance is configured without an access token file; canonical Fediverse links will be used");
                return None;
            }
        };
        let token = match fs::read_to_string(&token_file) {
            Ok(token) if !token.trim().is_empty() => token.trim().to_string(),
            Ok(_) => {
                tracing::warn!(%token_file, "Mastodon access token file is empty; canonical Fediverse links will be used");
                return None;
            }
            Err(error) => {
                tracing::warn!(%error, %token_file, "Failed to read Mastodon access token; canonical Fediverse links will be used");
                return None;
            }
        };

        match Self::new(&instance, token) {
            Ok(resolver) => Some(resolver),
            Err(error) => {
                tracing::warn!(%error, %instance, "Invalid preferred Mastodon instance; canonical Fediverse links will be used");
                None
            }
        }
    }

    fn new(instance: &str, token: String) -> Result<Self, Error> {
        let instance = mastodon_instance_url(instance)?;
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()?,
            instance,
            token,
        })
    }

    async fn resolve(&self, canonical_url: &Url) -> Result<Option<Url>, Error> {
        let search_url = self.instance.join("api/v2/search")?;
        let response = self
            .client
            .get(search_url)
            .bearer_auth(&self.token)
            .query(&[
                ("q", canonical_url.as_str()),
                ("type", "statuses"),
                ("resolve", "true"),
                ("limit", "1"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<MastodonSearch>()
            .await?;

        response
            .statuses
            .into_iter()
            .find(|status| status.uri == *canonical_url)
            .map(|status| mastodon_interaction_url(&self.instance, &status.uri))
            .transpose()
    }
}

fn mastodon_instance_url(instance: &str) -> Result<Url, Error> {
    let instance = if instance.contains("://") {
        Url::parse(instance)?
    } else {
        Url::parse(&format!("https://{instance}/"))?
    };
    if instance.scheme() != "https"
        || instance.host_str().is_none()
        || instance.path() != "/"
        || instance.query().is_some()
        || instance.fragment().is_some()
    {
        return Err(anyhow::anyhow!("Mastodon instance must be an HTTPS origin").into());
    }
    Ok(instance)
}

#[derive(Debug, FromRow)]
struct ActorRow {
    ap_id: String,
    username: String,
    inbox: String,
    shared_inbox: Option<String>,
    public_key: String,
    private_key: Option<String>,
    last_refreshed_at: DateTime<Utc>,
    local: bool,
}

#[derive(Clone, Debug)]
pub struct FediverseActor {
    username: String,
    ap_id: ObjectId<Self>,
    inbox: Url,
    shared_inbox: Option<Url>,
    public_key: String,
    private_key: Option<String>,
    last_refreshed_at: DateTime<Utc>,
    local: bool,
}

impl TryFrom<ActorRow> for FediverseActor {
    type Error = Error;

    fn try_from(row: ActorRow) -> Result<Self, Self::Error> {
        Ok(Self {
            username: row.username,
            ap_id: Url::parse(&row.ap_id)?.into(),
            inbox: Url::parse(&row.inbox)?,
            shared_inbox: row.shared_inbox.map(|url| Url::parse(&url)).transpose()?,
            public_key: row.public_key,
            private_key: row.private_key,
            last_refreshed_at: row.last_refreshed_at,
            local: row.local,
        })
    }
}

impl FediverseRepository {
    pub fn new(pool: PgPool, domain: String, username: String, blog_domain: String) -> Self {
        Self {
            pool,
            domain,
            username,
            blog_domain,
        }
    }

    fn actor_url(&self) -> Result<Url, Error> {
        Url::parse(&format!("https://{}/{}", self.domain, self.username)).map_err(Into::into)
    }

    async fn actor_by_id(&self, ap_id: &Url) -> Result<Option<FediverseActor>, Error> {
        sqlx::query_as::<_, ActorRow>(
            "SELECT ap_id, username, inbox, shared_inbox, public_key, private_key, last_refreshed_at, local FROM fediverse_actors WHERE ap_id = $1",
        )
        .bind(ap_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        .map(TryInto::try_into)
        .transpose()
    }

    async fn local_actor(&self) -> Result<FediverseActor, Error> {
        self.actor_by_id(&self.actor_url()?)
            .await?
            .ok_or_else(|| anyhow::anyhow!("local Fediverse actor is not initialized").into())
    }

    async fn save_actor(&self, actor: &FediverseActor) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO fediverse_actors (ap_id, username, inbox, shared_inbox, public_key, private_key, last_refreshed_at, local) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (ap_id) DO UPDATE SET username = EXCLUDED.username, inbox = EXCLUDED.inbox, shared_inbox = EXCLUDED.shared_inbox, public_key = EXCLUDED.public_key, last_refreshed_at = EXCLUDED.last_refreshed_at WHERE NOT fediverse_actors.local",
        )
        .bind(actor.ap_id.inner().as_str())
        .bind(&actor.username)
        .bind(actor.inbox.as_str())
        .bind(actor.shared_inbox.as_ref().map(Url::as_str))
        .bind(&actor.public_key)
        .bind(&actor.private_key)
        .bind(actor.last_refreshed_at)
        .bind(actor.local)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ensure_local_actor(&self) -> Result<FediverseActor, Error> {
        let ap_id = self.actor_url()?;
        if let Some(actor) = self.actor_by_id(&ap_id).await? {
            return Ok(actor);
        }

        let existing_id: Option<String> =
            sqlx::query_scalar("SELECT ap_id FROM fediverse_actors WHERE username = $1 AND local")
                .bind(&self.username)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(existing_id) = existing_id {
            return Err(anyhow::anyhow!(
                "Fediverse actor {} already exists at {}; configured domain is {}",
                self.username,
                existing_id,
                self.domain
            )
            .into());
        }

        let keypair = generate_actor_keypair()?;
        let actor = FediverseActor {
            username: self.username.clone(),
            inbox: actor_subresource_url(&ap_id, "inbox")?,
            ap_id: ap_id.into(),
            shared_inbox: None,
            public_key: keypair.public_key,
            private_key: Some(keypair.private_key),
            last_refreshed_at: Utc::now(),
            local: true,
        };
        self.save_actor(&actor).await?;
        Ok(actor)
    }

    pub async fn backfill_discussion_links(&self) -> Result<(), Error> {
        let actor = self.local_actor().await?;
        let slugs = sqlx::query_scalar::<_, String>(
            "SELECT slug FROM fediverse_published_posts ORDER BY published_at",
        )
        .fetch_all(&self.pool)
        .await?;

        for slug in slugs {
            let canonical_url = actor_post_url(actor.ap_id.inner(), &slug)?;
            sqlx::query(
                "UPDATE blog_post_discussion_links SET source = 'fediverse', label = 'Fediverse', url = $2 WHERE post_slug = $1 AND source = 'mastodon'",
            )
            .bind(&slug)
            .bind(canonical_url.as_str())
            .execute(&self.pool)
            .await?;
            sqlx::query(
                "INSERT INTO blog_post_discussion_links (post_slug, source, label, url) SELECT $1, 'fediverse', 'Fediverse', $2 WHERE NOT EXISTS (SELECT 1 FROM blog_post_discussion_links WHERE post_slug = $1 AND source IN ('fediverse', 'mastodon')) ON CONFLICT DO NOTHING",
            )
            .bind(slug)
            .bind(canonical_url.as_str())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn add_follower(
        &self,
        local: &FediverseActor,
        follower: &FediverseActor,
    ) -> Result<(), Error> {
        self.save_actor(follower).await?;
        sqlx::query(
            "INSERT INTO fediverse_followers (local_actor_id, follower_actor_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(local.ap_id.inner().as_str())
        .bind(follower.ap_id.inner().as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_follower(&self, local: &Url, follower: &Url) -> Result<(), Error> {
        sqlx::query(
            "DELETE FROM fediverse_followers WHERE local_actor_id = $1 AND follower_actor_id = $2",
        )
        .bind(local.as_str())
        .bind(follower.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn followers(&self, actor: &FediverseActor) -> Result<Vec<FediverseActor>, Error> {
        sqlx::query_as::<_, ActorRow>(
            "SELECT a.ap_id, a.username, a.inbox, a.shared_inbox, a.public_key, a.private_key, a.last_refreshed_at, a.local FROM fediverse_actors a JOIN fediverse_followers f ON f.follower_actor_id = a.ap_id WHERE f.local_actor_id = $1 ORDER BY f.followed_at",
        )
        .bind(actor.ap_id.inner().as_str())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "type")]
    kind: String,
    preferred_username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    id: ObjectId<FediverseActor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<Image>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<Image>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discoverable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    indexable: Option<bool>,
    inbox: Url,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    endpoints: Option<Endpoints>,
    public_key: PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    followers: Option<Url>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Image {
    #[serde(rename = "type")]
    kind: String,
    media_type: String,
    url: Url,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Endpoints {
    shared_inbox: Url,
}

#[async_trait::async_trait]
impl Object for FediverseActor {
    type DataType = FediverseRepository;
    type Kind = Person;
    type Error = Error;

    fn id(&self) -> &Url {
        self.ap_id.inner()
    }

    fn last_refreshed_at(&self) -> Option<DateTime<Utc>> {
        Some(self.last_refreshed_at)
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        data.actor_by_id(&object_id).await
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let followers = self
            .local
            .then(|| actor_subresource_url(self.ap_id.inner(), "followers"))
            .transpose()?;
        Ok(Person {
            kind: if self.local { "Service" } else { "Person" }.to_string(),
            preferred_username: self.username.clone(),
            name: self.local.then(|| "FlakM blog".to_string()),
            id: self.ap_id.clone(),
            url: self
                .local
                .then(|| Url::parse(&format!("https://{}/", data.blog_domain)))
                .transpose()?,
            icon: self
                .local
                .then(|| {
                    Ok::<_, url::ParseError>(Image {
                        kind: "Image".to_string(),
                        media_type: "image/jpeg".to_string(),
                        url: Url::parse(&format!(
                            "https://{}/images/avatar.jpg",
                            data.blog_domain
                        ))?,
                        name: Some("Portrait of Maciek Flak".to_string()),
                    })
                })
                .transpose()?,
            image: self
                .local
                .then(|| {
                    Ok::<_, url::ParseError>(Image {
                        kind: "Image".to_string(),
                        media_type: "image/png".to_string(),
                        url: Url::parse(&format!(
                            "https://{}/images/fediverse-header.png",
                            data.blog_domain
                        ))?,
                        name: Some("FlakM blog homepage in its dark theme".to_string()),
                    })
                })
                .transpose()?,
            discoverable: self.local.then_some(true),
            indexable: self.local.then_some(true),
            inbox: self.inbox.clone(),
            endpoints: self
                .shared_inbox
                .clone()
                .map(|shared_inbox| Endpoints { shared_inbox }),
            public_key: self.public_key(),
            followers,
            summary: self.local.then(|| {
                format!(
                    "<p>Technical notes by Maciek Flak about Rust, systems, observability, and the tools around them. Follow for new posts or browse the archive at <a href=\"https://{0}/\" rel=\"me\">{0}</a>.</p>",
                    data.blog_domain
                )
            }),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        if json.kind != "Person" && json.kind != "Service" && json.kind != "Application" {
            return Err(anyhow::anyhow!("unsupported actor type {}", json.kind).into());
        }
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let actor = Self {
            username: json.preferred_username,
            ap_id: json.id,
            inbox: json.inbox,
            shared_inbox: json.endpoints.map(|endpoints| endpoints.shared_inbox),
            public_key: json.public_key.public_key_pem,
            private_key: None,
            last_refreshed_at: Utc::now(),
            local: false,
        };
        data.save_actor(&actor).await?;
        Ok(actor)
    }
}

impl Actor for FediverseActor {
    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone()
    }

    fn inbox(&self) -> Url {
        self.inbox.clone()
    }

    fn shared_inbox(&self) -> Option<Url> {
        self.shared_inbox.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Follow {
    actor: ObjectId<FediverseActor>,
    object: ObjectId<FediverseActor>,
    #[serde(rename = "type")]
    kind: String,
    id: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UndoFollow {
    actor: ObjectId<FediverseActor>,
    object: Follow,
    #[serde(rename = "type")]
    kind: String,
    id: Url,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum AcceptedActivity {
    Follow(Follow),
    UndoFollow(UndoFollow),
}

#[async_trait::async_trait]
impl Activity for AcceptedActivity {
    type DataType = FediverseRepository;
    type Error = Error;

    fn id(&self) -> &Url {
        match self {
            Self::Follow(activity) => &activity.id,
            Self::UndoFollow(activity) => &activity.id,
        }
    }

    fn actor(&self) -> &Url {
        match self {
            Self::Follow(activity) => activity.actor.inner(),
            Self::UndoFollow(activity) => activity.actor.inner(),
        }
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let local_actor = data.local_actor().await?;
        match self {
            Self::Follow(activity) => {
                if activity.kind != "Follow" || activity.object.inner() != local_actor.ap_id.inner()
                {
                    return Err(anyhow::anyhow!("invalid Follow target").into());
                }
            }
            Self::UndoFollow(activity) => {
                if activity.kind != "Undo"
                    || activity.object.kind != "Follow"
                    || activity.actor.inner() != activity.object.actor.inner()
                    || activity.object.object.inner() != local_actor.ap_id.inner()
                {
                    return Err(anyhow::anyhow!("invalid Undo Follow target").into());
                }
            }
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let local_actor = data.local_actor().await?;
        match self {
            Self::Follow(follow) => {
                let follower = follow.actor.dereference(data).await?;
                data.add_follower(&local_actor, &follower).await?;
                send_accept(&local_actor, follow, &follower, data).await
            }
            Self::UndoFollow(undo) => {
                let follower = undo.actor.dereference(data).await?;
                data.remove_follower(local_actor.ap_id.inner(), follower.ap_id.inner())
                    .await?;
                send_accept(&local_actor, undo.object, &follower, data).await
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Accept {
    actor: ObjectId<FediverseActor>,
    object: Follow,
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateActor {
    actor: ObjectId<FediverseActor>,
    object: Person,
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
}

#[async_trait::async_trait]
impl Activity for UpdateActor {
    type DataType = FediverseRepository;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }
    fn actor(&self) -> &Url {
        self.actor.inner()
    }
    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Activity for Accept {
    type DataType = FediverseRepository;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }
    fn actor(&self) -> &Url {
        self.actor.inner()
    }
    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
}

async fn send_accept(
    local_actor: &FediverseActor,
    follow: Follow,
    follower: &FediverseActor,
    data: &Data<FediverseRepository>,
) -> Result<(), Error> {
    let accept = WithContext::new_default(Accept {
        actor: local_actor.ap_id.clone(),
        object: follow,
        kind: "Accept",
        id: activity_url(data.domain())?,
    });
    send_activity(
        &accept,
        local_actor,
        vec![follower.shared_inbox_or_inbox()],
        data,
    )
    .await
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Note {
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
    attributed_to: ObjectId<FediverseActor>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    to: Vec<Url>,
    content: String,
    url: Url,
    published: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated: Option<DateTime<Utc>>,
    icon: Image,
    attachment: Vec<Image>,
    tag: Vec<Hashtag>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Hashtag {
    #[serde(rename = "type")]
    kind: &'static str,
    name: String,
    href: Url,
}

#[derive(Debug, Serialize)]
struct PostActivity {
    actor: ObjectId<FediverseActor>,
    to: Vec<Url>,
    object: Note,
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
}

#[async_trait::async_trait]
impl Activity for PostActivity {
    type DataType = FediverseRepository;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }
    fn actor(&self) -> &Url {
        self.actor.inner()
    }
    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn note_for_post(actor: &FediverseActor, post: &HugoBlogPost) -> Result<Note, Error> {
    let id = actor_post_url(actor.ap_id.inner(), &post.slug)?;
    let image = image_for_post(post)?;
    Ok(Note {
        kind: "Note",
        id: id.clone(),
        attributed_to: actor.ap_id.clone(),
        to: vec![Url::parse("https://www.w3.org/ns/activitystreams#Public")?],
        content: format!(
            "<p><strong>{}</strong></p><p>{}</p><p><a href=\"{}\">Read more</a></p>",
            escape_html(&post.title),
            escape_html(&post.description),
            escape_html(post.url.as_str())
        ),
        url: id,
        published: post.date,
        updated: None,
        icon: image.clone(),
        attachment: vec![image],
        tag: post
            .tags
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|tag| {
                Ok(Hashtag {
                    kind: "Hashtag",
                    name: format!("#{tag}"),
                    href: tag_url(&post.url, &tag)?,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

fn image_for_post(post: &HugoBlogPost) -> Result<Image, Error> {
    let path = post
        .featured_image
        .as_deref()
        .unwrap_or("/images/fediverse-post.png");
    let url = match Url::parse(path) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => {
            let mut base = post.url.clone();
            base.set_path("/");
            base.set_query(None);
            base.set_fragment(None);
            base.join(path.trim_start_matches('/'))?
        }
    };
    let media_type = match url.path().rsplit('.').next() {
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        _ => "image/png",
    };
    Ok(Image {
        kind: "Image".to_string(),
        media_type: media_type.to_string(),
        url,
        name: Some(post.title.clone()),
    })
}

async fn send_activity<A>(
    activity: &A,
    actor: &FediverseActor,
    inboxes: Vec<Url>,
    data: &Data<FediverseRepository>,
) -> Result<(), Error>
where
    A: Activity<DataType = FediverseRepository, Error = Error> + Serialize + Debug,
{
    let mut unique = HashSet::new();
    for task in SendActivityTask::prepare(
        activity,
        actor,
        inboxes
            .into_iter()
            .filter(|url| unique.insert(url.clone()))
            .collect(),
        data,
    )
    .await?
    {
        task.sign_and_send(data).await?;
    }
    Ok(())
}

pub async fn refresh_follower_profiles(
    repository: FediverseRepository,
    config: FederationConfig<FediverseRepository>,
) {
    let data = config.to_request_data();
    let result: Result<(), Error> = async {
        let actor = repository.local_actor().await?;
        let inboxes = repository
            .followers(&actor)
            .await?
            .iter()
            .map(Actor::shared_inbox_or_inbox)
            .collect::<Vec<_>>();
        if inboxes.is_empty() {
            return Ok(());
        }
        let activity = WithContext::new_default(UpdateActor {
            actor: actor.ap_id.clone(),
            object: actor.clone().into_json(&data).await?,
            kind: "Update",
            id: activity_url(data.domain())?,
        });
        send_activity(&activity, &actor, inboxes, &data).await?;
        tracing::info!("Refreshed Fediverse profile metadata");
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, "Failed to refresh Fediverse profile metadata");
    }
}

pub async fn refresh_post_media(
    repository: FediverseRepository,
    blog_repository: BlogRepository,
    config: FederationConfig<FediverseRepository>,
) {
    let data = config.to_request_data();
    let result: Result<(), Error> = async {
        let actor = repository.local_actor().await?;
        let inboxes = repository
            .followers(&actor)
            .await?
            .iter()
            .map(Actor::shared_inbox_or_inbox)
            .collect::<Vec<_>>();
        if inboxes.is_empty() {
            return Ok(());
        }
        for post in blog_repository.fediverse_posts_without_media().await? {
            let mut note = note_for_post(&actor, &post)?;
            note.updated = Some(Utc::now());
            let activity = WithContext::new_default(PostActivity {
                actor: actor.ap_id.clone(),
                to: note.to.clone(),
                object: note,
                kind: "Update",
                id: activity_url(data.domain())?,
            });
            send_activity(&activity, &actor, inboxes.clone(), &data).await?;
            blog_repository
                .record_fediverse_media_refresh(&post.slug)
                .await?;
            tracing::info!(post_slug = %post.slug, "Added media to existing Fediverse post");
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, "Failed to refresh Fediverse post media");
    }
}

pub async fn publish_new_posts(
    repository: FediverseRepository,
    blog_repository: BlogRepository,
    config: FederationConfig<FediverseRepository>,
    mastodon_resolver: Option<MastodonResolver>,
) {
    tokio::time::sleep(Duration::from_secs(5)).await;
    let data = config.to_request_data();
    let result: Result<(), Error> = async {
        let actor = repository.local_actor().await?;
        let followers = repository.followers(&actor).await?;
        let inboxes = followers
            .iter()
            .map(Actor::shared_inbox_or_inbox)
            .collect::<Vec<_>>();
        for post in blog_repository.unpublished_fediverse_posts().await? {
            let note = note_for_post(&actor, &post)?;
            let activity = WithContext::new_default(PostActivity {
                actor: actor.ap_id.clone(),
                to: note.to.clone(),
                object: note,
                kind: "Create",
                id: activity_url(data.domain())?,
            });
            send_activity(&activity, &actor, inboxes.clone(), &data).await?;
            blog_repository
                .record_fediverse_publication(&post.slug, &activity.inner().object.id)
                .await?;
            tracing::info!(post_slug = %post.slug, "Published post to Fediverse");
            if let Some(resolver) = &mastodon_resolver {
                resolve_mastodon_discussion(
                    resolver,
                    &blog_repository,
                    &post.slug,
                    &activity.inner().object.id,
                )
                .await;
            }
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::error!(%error, "Failed to publish posts to Fediverse");
    }
}

pub async fn resolve_existing_discussion_links(
    resolver: MastodonResolver,
    blog_repository: BlogRepository,
) {
    let fallbacks = match blog_repository.fediverse_fallbacks().await {
        Ok(fallbacks) => fallbacks,
        Err(error) => {
            tracing::warn!(%error, "Failed to load Fediverse discussion fallbacks");
            return;
        }
    };
    for (slug, canonical_url) in fallbacks {
        resolve_mastodon_discussion(&resolver, &blog_repository, &slug, &canonical_url).await;
    }
}

async fn resolve_mastodon_discussion(
    resolver: &MastodonResolver,
    blog_repository: &BlogRepository,
    slug: &str,
    canonical_url: &Url,
) {
    let mut last_error = None;
    for attempt in 1..=10 {
        match resolver.resolve(canonical_url).await {
            Ok(Some(thread_url)) => {
                if let Err(error) = blog_repository
                    .record_mastodon_discussion(slug, &thread_url)
                    .await
                {
                    tracing::warn!(%error, post_slug = %slug, "Failed to persist Mastodon discussion link");
                } else {
                    tracing::info!(post_slug = %slug, %thread_url, "Resolved Mastodon discussion link");
                }
                return;
            }
            Ok(None) => last_error = None,
            Err(error) => last_error = Some(error),
        }
        if attempt < 10 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    if let Some(error) = last_error {
        tracing::warn!(%error, post_slug = %slug, %canonical_url, "Mastodon discussion resolution failed; keeping canonical Fediverse link");
    } else {
        tracing::warn!(post_slug = %slug, %canonical_url, "Mastodon status was not found; keeping canonical Fediverse link");
    }
}

pub fn router(config: FederationConfig<FediverseRepository>) -> Router<PgPool> {
    Router::<PgPool>::new()
        .route("/.well-known/webfinger", get(webfinger))
        .route("/{user}", get(get_actor))
        .route("/{user}/inbox", post(post_inbox))
        .route("/{user}/followers", get(get_followers))
        .route("/{user}/posts/{slug}", get(get_post))
        .layer(FederationMiddleware::new(config))
}

async fn get_actor(
    Path(name): Path<String>,
    data: Data<FediverseRepository>,
) -> Result<FederationJson<WithContext<Person>>, StatusCode> {
    if name != data.username {
        return Err(StatusCode::NOT_FOUND);
    }
    let actor = data.local_actor().await.map_err(internal_error)?;
    let person = actor.into_json(&data).await.map_err(internal_error)?;
    Ok(FederationJson(WithContext::new_default(person)))
}

async fn post_inbox(data: Data<FediverseRepository>, activity_data: ActivityData) -> Response {
    match receive_activity::<WithContext<AcceptedActivity>, FediverseActor, FediverseRepository>(
        activity_data,
        &data,
    )
    .await
    {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(error) => {
            tracing::warn!(?error, "Rejected Fediverse activity");
            StatusCode::BAD_REQUEST.into_response()
        }
    }
}

#[derive(Deserialize)]
struct WebfingerQuery {
    resource: String,
}

async fn webfinger(
    Query(query): Query<WebfingerQuery>,
    data: Data<FediverseRepository>,
) -> Result<Json<Webfinger>, StatusCode> {
    let name = extract_webfinger_name(&query.resource, &data).map_err(|_| StatusCode::NOT_FOUND)?;
    if name != data.username {
        return Err(StatusCode::NOT_FOUND);
    }
    let actor = data.local_actor().await.map_err(internal_error)?;
    Ok(Json(build_webfinger_response(
        query.resource,
        actor.ap_id.into_inner(),
    )))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FollowersCollection {
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
    total_items: usize,
    ordered_items: Vec<Url>,
}

async fn get_followers(
    Path(name): Path<String>,
    data: Data<FediverseRepository>,
) -> Result<FederationJson<WithContext<FollowersCollection>>, StatusCode> {
    if name != data.username {
        return Err(StatusCode::NOT_FOUND);
    }
    let actor = data.local_actor().await.map_err(internal_error)?;
    let followers = data.followers(&actor).await.map_err(internal_error)?;
    let collection = FollowersCollection {
        kind: "OrderedCollection",
        id: actor_subresource_url(actor.ap_id.inner(), "followers").map_err(internal_error)?,
        total_items: followers.len(),
        ordered_items: followers
            .into_iter()
            .map(|actor| actor.ap_id.into_inner())
            .collect(),
    };
    Ok(FederationJson(WithContext::new_default(collection)))
}

async fn get_post(
    Path((name, slug)): Path<(String, String)>,
    data: Data<FediverseRepository>,
) -> Result<FederationJson<WithContext<Note>>, StatusCode> {
    if name != data.username {
        return Err(StatusCode::NOT_FOUND);
    }
    let post = BlogRepository {
        db: data.pool.clone(),
    }
    .by_slug(&slug)
    .await
    .map_err(|error| internal_error(error.into()))?
    .ok_or(StatusCode::NOT_FOUND)?;
    let actor = data.local_actor().await.map_err(internal_error)?;
    let note = note_for_post(&actor, &post).map_err(internal_error)?;
    Ok(FederationJson(WithContext::new_default(note)))
}

fn internal_error(error: Error) -> StatusCode {
    tracing::error!(%error, "Fediverse request failed");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn activity_url(domain: &str) -> Result<Url, Error> {
    Url::parse(&format!("https://{domain}/activities/{}", Uuid::new_v4())).map_err(Into::into)
}

fn actor_subresource_url(actor: &Url, resource: &str) -> Result<Url, Error> {
    let mut url = actor.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("actor URL cannot be a base URL"))?
        .push(resource);
    Ok(url)
}

fn actor_post_url(actor: &Url, slug: &str) -> Result<Url, Error> {
    let mut url = actor.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("actor URL cannot be a base URL"))?
        .push("posts")
        .push(slug);
    Ok(url)
}

fn mastodon_interaction_url(instance: &Url, status_url: &Url) -> Result<Url, Error> {
    let mut url = instance.join("authorize_interaction")?;
    url.query_pairs_mut()
        .append_pair("uri", status_url.as_str());
    Ok(url)
}

fn tag_url(post_url: &Url, tag: &str) -> Result<Url, Error> {
    let mut url = post_url.clone();
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("post URL cannot be a base URL"))?
        .push("tags")
        .push(tag);
    Ok(url)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> FediverseActor {
        let ap_id = Url::parse("https://fedi.flakm.com/blog").unwrap();
        FediverseActor {
            username: "blog".to_string(),
            inbox: actor_subresource_url(&ap_id, "inbox").unwrap(),
            ap_id: ap_id.into(),
            shared_inbox: None,
            public_key: "public key".to_string(),
            private_key: Some("private key".to_string()),
            last_refreshed_at: Utc::now(),
            local: true,
        }
    }

    fn post() -> HugoBlogPost {
        HugoBlogPost {
            title: "Rust <and> ActivityPub".to_string(),
            slug: "rust-fedi".to_string(),
            description: "Signed & delivered".to_string(),
            date: DateTime::parse_from_rfc3339("2026-08-24T12:00:00Z")
                .unwrap()
                .into(),
            featured_image: None,
            tags: Some(vec!["rust lang".to_string()]),
            url: Url::parse("https://flakm.com/posts/rust-fedi/").unwrap(),
        }
    }

    #[test]
    fn note_has_stable_id_safe_html_and_encoded_tag_url() {
        let note = note_for_post(&actor(), &post()).unwrap();
        assert_eq!(
            note.id.as_str(),
            "https://fedi.flakm.com/blog/posts/rust-fedi"
        );
        assert_eq!(note.url, note.id);
        assert!(note.content.contains("Rust &lt;and&gt; ActivityPub"));
        assert!(note.content.contains("Signed &amp; delivered"));
        assert!(note.content.contains("https://flakm.com/posts/rust-fedi/"));
        assert_eq!(
            note.icon.url.as_str(),
            "https://flakm.com/images/fediverse-post.png"
        );
        assert_eq!(note.attachment, vec![note.icon.clone()]);
        assert_eq!(
            note.tag[0].href.as_str(),
            "https://flakm.com/tags/rust%20lang"
        );
    }

    #[test]
    fn actor_subresources_preserve_actor_path() {
        let actor = actor();
        assert_eq!(actor.inbox.as_str(), "https://fedi.flakm.com/blog/inbox");
        assert_eq!(
            actor_subresource_url(actor.ap_id.inner(), "followers")
                .unwrap()
                .as_str(),
            "https://fedi.flakm.com/blog/followers"
        );
    }

    #[test]
    fn mastodon_interaction_uses_canonical_status_url() {
        let status = Url::parse("https://fedi.flakm.com/blog/posts/example").unwrap();
        let instance = Url::parse("https://hachyderm.io/").unwrap();

        assert_eq!(
            mastodon_interaction_url(&instance, &status)
                .unwrap()
                .as_str(),
            "https://hachyderm.io/authorize_interaction?uri=https%3A%2F%2Ffedi.flakm.com%2Fblog%2Fposts%2Fexample"
        );
    }

    #[test]
    fn mastodon_resolver_requires_https_origin() {
        assert!(mastodon_instance_url("hachyderm.io").is_ok());
        assert!(mastodon_instance_url("http://hachyderm.io").is_err());
        assert!(mastodon_instance_url("https://hachyderm.io/path").is_err());
    }
}
