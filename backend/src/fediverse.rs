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
    body::{to_bytes, Body},
    extract::{FromRequest, Path, Query, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
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
    display_name: Option<String>,
    profile_url: Option<String>,
    avatar_url: Option<String>,
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
    display_name: Option<String>,
    profile_url: Option<Url>,
    avatar_url: Option<Url>,
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
            display_name: row.display_name,
            profile_url: row.profile_url.map(|url| Url::parse(&url)).transpose()?,
            avatar_url: row.avatar_url.map(|url| Url::parse(&url)).transpose()?,
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
            "SELECT ap_id, username, display_name, profile_url, avatar_url, inbox, shared_inbox, public_key, private_key, last_refreshed_at, local FROM fediverse_actors WHERE ap_id = $1",
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
            "INSERT INTO fediverse_actors (ap_id, username, display_name, profile_url, avatar_url, inbox, shared_inbox, public_key, private_key, last_refreshed_at, local) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ON CONFLICT (ap_id) DO UPDATE SET username = EXCLUDED.username, display_name = EXCLUDED.display_name, profile_url = EXCLUDED.profile_url, avatar_url = EXCLUDED.avatar_url, inbox = EXCLUDED.inbox, shared_inbox = EXCLUDED.shared_inbox, public_key = EXCLUDED.public_key, last_refreshed_at = EXCLUDED.last_refreshed_at WHERE NOT fediverse_actors.local",
        )
        .bind(actor.ap_id.inner().as_str())
        .bind(&actor.username)
        .bind(&actor.display_name)
        .bind(actor.profile_url.as_ref().map(Url::as_str))
        .bind(actor.avatar_url.as_ref().map(Url::as_str))
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
            display_name: None,
            profile_url: None,
            avatar_url: None,
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
            "SELECT a.ap_id, a.username, a.display_name, a.profile_url, a.avatar_url, a.inbox, a.shared_inbox, a.public_key, a.private_key, a.last_refreshed_at, a.local FROM fediverse_actors a JOIN fediverse_followers f ON f.follower_actor_id = a.ap_id WHERE f.local_actor_id = $1 ORDER BY f.followed_at",
        )
        .bind(actor.ap_id.inner().as_str())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
            .collect()
    }

    async fn published_post_slug(&self, object: &Url) -> Result<Option<String>, Error> {
        let actor = self.actor_url()?;
        let prefix = format!("{}/posts/", actor.as_str().trim_end_matches('/'));
        let Some(slug) = object.as_str().strip_prefix(&prefix) else {
            return Ok(None);
        };
        if slug.is_empty() || slug.contains('/') || actor_post_url(&actor, slug)? != *object {
            return Ok(None);
        }
        Ok(
            sqlx::query_scalar("SELECT slug FROM fediverse_published_posts WHERE slug = $1")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn reply_post_slug(&self, object: &Url) -> Result<Option<String>, Error> {
        if let Some(slug) = self.published_post_slug(object).await? {
            return Ok(Some(slug));
        }
        Ok(sqlx::query_scalar(
            "SELECT post_slug FROM fediverse_replies WHERE object_id = $1 AND deleted_at IS NULL",
        )
        .bind(object.as_str())
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn direct_reply_ids(&self, slug: &str, root: &Url) -> Result<Vec<Url>, Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT object_id FROM fediverse_replies WHERE post_slug = $1 AND deleted_at IS NULL AND (in_reply_to = $2 OR in_reply_to IS NULL) ORDER BY published_at, object_id",
        )
        .bind(slug)
        .bind(root.as_str())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|id| Url::parse(&id).map_err(Into::into))
            .collect()
    }

    async fn reply_actor_ids(&self) -> Result<Vec<Url>, Error> {
        sqlx::query_scalar::<_, String>("SELECT DISTINCT actor_id FROM fediverse_replies")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|id| Url::parse(&id).map_err(Into::into))
            .collect()
    }

    async fn record_received(
        transaction: &mut Transaction<'_, Postgres>,
        activity_id: &Url,
        actor_id: &Url,
        activity_type: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "INSERT INTO fediverse_received_activities (activity_id, actor_id, activity_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(activity_id.as_str())
        .bind(actor_id.as_str())
        .bind(activity_type)
        .execute(&mut **transaction)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    async fn record_reaction(
        &self,
        activity_id: &Url,
        actor_id: &Url,
        object_id: &Url,
        kind: &str,
    ) -> Result<(), Error> {
        let slug = self
            .published_post_slug(object_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("reaction does not target a published local post"))?;
        let mut transaction = self.pool.begin().await?;
        if !Self::record_received(&mut transaction, activity_id, actor_id, kind).await? {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO fediverse_reactions (activity_id, post_slug, actor_id, kind, object_id) SELECT $1, $2, $3, $4, $5 WHERE NOT EXISTS (SELECT 1 FROM fediverse_undone_activities WHERE activity_id = $1 AND actor_id = $3) ON CONFLICT (post_slug, actor_id, kind) DO UPDATE SET activity_id = EXCLUDED.activity_id, object_id = EXCLUDED.object_id, created_at = NOW()",
        )
        .bind(activity_id.as_str())
        .bind(slug)
        .bind(actor_id.as_str())
        .bind(kind)
        .bind(object_id.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn undo_reaction(
        &self,
        undo_id: &Url,
        actor_id: &Url,
        reaction_id: &Url,
        kind: Option<&str>,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        if !Self::record_received(&mut transaction, undo_id, actor_id, "Undo").await? {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO fediverse_undone_activities (activity_id, actor_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(reaction_id.as_str())
        .bind(actor_id.as_str())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM fediverse_reactions WHERE activity_id = $1 AND actor_id = $2 AND ($3::VARCHAR IS NULL OR kind = $3)",
        )
        .bind(reaction_id.as_str())
        .bind(actor_id.as_str())
        .bind(kind)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn record_reply(
        &self,
        activity_id: &Url,
        actor_id: &Url,
        note: &RemoteNote,
    ) -> Result<(), Error> {
        let slug = self
            .reply_post_slug(&note.in_reply_to)
            .await?
            .ok_or_else(|| anyhow::anyhow!("reply does not target a published local post"))?;
        let mut transaction = self.pool.begin().await?;
        if !Self::record_received(&mut transaction, activity_id, actor_id, "Create").await? {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO fediverse_replies (object_id, create_activity_id, post_slug, actor_id, url, content, published_at, in_reply_to) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (object_id) DO NOTHING",
        )
        .bind(note.id.as_str())
        .bind(activity_id.as_str())
        .bind(slug)
        .bind(actor_id.as_str())
        .bind(note.url.as_ref().unwrap_or(&note.id).as_str())
        .bind(&note.content)
        .bind(note.published.unwrap_or_else(Utc::now))
        .bind(note.in_reply_to.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn reply_owner(&self, object_id: &Url) -> Result<Option<String>, Error> {
        Ok(
            sqlx::query_scalar("SELECT actor_id FROM fediverse_replies WHERE object_id = $1")
                .bind(object_id.as_str())
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn update_reply(
        &self,
        activity_id: &Url,
        actor_id: &Url,
        note: &RemoteNote,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        if !Self::record_received(&mut transaction, activity_id, actor_id, "Update").await? {
            return Ok(());
        }
        sqlx::query(
            "UPDATE fediverse_replies SET url = $3, content = $4, updated_at = $5 WHERE object_id = $1 AND actor_id = $2 AND deleted_at IS NULL AND (updated_at IS NULL OR updated_at < $5)",
        )
        .bind(note.id.as_str())
        .bind(actor_id.as_str())
        .bind(note.url.as_ref().unwrap_or(&note.id).as_str())
        .bind(&note.content)
        .bind(note.updated.unwrap_or_else(Utc::now))
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn delete_reply(
        &self,
        activity_id: &Url,
        actor_id: &Url,
        object_id: &Url,
        activity_type: &str,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        if !Self::record_received(&mut transaction, activity_id, actor_id, activity_type).await? {
            return Ok(());
        }
        sqlx::query(
            "UPDATE fediverse_replies SET deleted_at = NOW() WHERE object_id = $1 AND actor_id = $2 AND deleted_at IS NULL",
        )
        .bind(object_id.as_str())
        .bind(actor_id.as_str())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn record_unsupported(
        &self,
        activity_id: &Url,
        actor_id: &Url,
        kind: &str,
    ) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;
        Self::record_received(&mut transaction, activity_id, actor_id, kind).await?;
        transaction.commit().await?;
        Ok(())
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
            display_name: json.name,
            profile_url: json.url,
            avatar_url: json.icon.map(|icon| icon.url),
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteNote {
    #[serde(rename = "type")]
    kind: String,
    id: Url,
    attributed_to: ObjectId<FediverseActor>,
    in_reply_to: Url,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    to: Vec<Url>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    cc: Vec<Url>,
    content: String,
    #[serde(default)]
    url: Option<Url>,
    #[serde(default)]
    published: Option<DateTime<Utc>>,
    #[serde(default)]
    updated: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize)]
struct DeletedObject {
    id: Url,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum DeleteTarget {
    Object(DeletedObject),
    Id(Url),
}

impl DeleteTarget {
    fn id(&self) -> &Url {
        match self {
            Self::Object(object) => &object.id,
            Self::Id(id) => id,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
enum UndoObject {
    Follow {
        actor: ObjectId<FediverseActor>,
        object: ObjectId<FediverseActor>,
    },
    Like {
        actor: ObjectId<FediverseActor>,
        object: Url,
        id: Url,
    },
    Announce {
        actor: ObjectId<FediverseActor>,
        object: Url,
        id: Url,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum UndoTarget {
    Object(UndoObject),
    Id(Url),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum KnownActivity {
    Follow {
        actor: ObjectId<FediverseActor>,
        object: ObjectId<FediverseActor>,
        id: Url,
    },
    Undo {
        actor: ObjectId<FediverseActor>,
        object: UndoTarget,
        id: Url,
    },
    Like {
        actor: ObjectId<FediverseActor>,
        object: Url,
        id: Url,
    },
    Announce {
        actor: ObjectId<FediverseActor>,
        object: Url,
        id: Url,
    },
    Create {
        actor: ObjectId<FediverseActor>,
        object: Box<RemoteNote>,
        id: Url,
    },
    Update {
        actor: ObjectId<FediverseActor>,
        object: Box<RemoteNote>,
        id: Url,
    },
    Delete {
        actor: ObjectId<FediverseActor>,
        object: DeleteTarget,
        id: Url,
    },
}

impl KnownActivity {
    fn id(&self) -> &Url {
        match self {
            Self::Follow { id, .. }
            | Self::Undo { id, .. }
            | Self::Like { id, .. }
            | Self::Announce { id, .. }
            | Self::Create { id, .. }
            | Self::Update { id, .. }
            | Self::Delete { id, .. } => id,
        }
    }

    fn actor(&self) -> &Url {
        match self {
            Self::Follow { actor, .. }
            | Self::Undo { actor, .. }
            | Self::Like { actor, .. }
            | Self::Announce { actor, .. }
            | Self::Create { actor, .. }
            | Self::Update { actor, .. }
            | Self::Delete { actor, .. } => actor.inner(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UnsupportedActivity {
    actor: ObjectId<FediverseActor>,
    #[serde(rename = "type")]
    kind: String,
    id: Url,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AcceptedActivity {
    Known(KnownActivity),
    Unsupported(UnsupportedActivity),
}

#[async_trait::async_trait]
impl Activity for AcceptedActivity {
    type DataType = FediverseRepository;
    type Error = Error;

    fn id(&self) -> &Url {
        match self {
            Self::Known(activity) => activity.id(),
            Self::Unsupported(activity) => &activity.id,
        }
    }

    fn actor(&self) -> &Url {
        match self {
            Self::Known(activity) => activity.actor(),
            Self::Unsupported(activity) => activity.actor.inner(),
        }
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let local_actor = data.local_actor().await?;
        match self {
            Self::Known(KnownActivity::Follow { object, .. }) => {
                if object.inner() != local_actor.ap_id.inner() {
                    return Err(anyhow::anyhow!("invalid Follow target").into());
                }
            }
            Self::Known(KnownActivity::Undo { actor, object, .. }) => match object {
                UndoTarget::Object(UndoObject::Follow {
                    actor: followed_by,
                    object,
                    ..
                }) => {
                    if actor.inner() != followed_by.inner()
                        || object.inner() != local_actor.ap_id.inner()
                    {
                        return Err(anyhow::anyhow!("invalid Undo Follow target").into());
                    }
                }
                UndoTarget::Object(UndoObject::Like {
                    actor: liked_by,
                    object,
                    ..
                })
                | UndoTarget::Object(UndoObject::Announce {
                    actor: liked_by,
                    object,
                    ..
                }) => {
                    if actor.inner() != liked_by.inner()
                        || data.published_post_slug(object).await?.is_none()
                    {
                        return Err(anyhow::anyhow!("invalid Undo reaction target").into());
                    }
                }
                UndoTarget::Id(_) => {}
            },
            Self::Known(KnownActivity::Like { object, .. })
            | Self::Known(KnownActivity::Announce { object, .. }) => {
                if data.published_post_slug(object).await?.is_none() {
                    return Err(anyhow::anyhow!("invalid reaction target").into());
                }
            }
            Self::Known(KnownActivity::Create { actor, object, .. }) => {
                verify_remote_note(actor.inner(), object, data, false).await?;
            }
            Self::Known(KnownActivity::Update { actor, object, .. }) => {
                verify_remote_note(actor.inner(), object, data, true).await?;
                if data
                    .reply_owner(&object.id)
                    .await?
                    .is_some_and(|owner| owner != actor.inner().as_str())
                {
                    return Err(anyhow::anyhow!("cannot update another actor's reply").into());
                }
            }
            Self::Known(KnownActivity::Delete { actor, object, .. }) => {
                if data
                    .reply_owner(object.id())
                    .await?
                    .is_some_and(|owner| owner != actor.inner().as_str())
                {
                    return Err(anyhow::anyhow!("cannot delete another actor's reply").into());
                }
            }
            Self::Unsupported(_) => {}
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let local_actor = data.local_actor().await?;
        match self {
            Self::Known(KnownActivity::Follow { actor, object, id }) => {
                let follower = actor.dereference(data).await?;
                let follow = Follow {
                    actor: follower.ap_id.clone(),
                    object,
                    kind: "Follow".to_string(),
                    id,
                };
                data.add_follower(&local_actor, &follower).await?;
                send_accept(&local_actor, follow, &follower, data).await
            }
            Self::Known(KnownActivity::Undo { actor, object, id }) => match object {
                UndoTarget::Object(UndoObject::Follow { .. }) => {
                    data.remove_follower(local_actor.ap_id.inner(), actor.inner())
                        .await
                }
                UndoTarget::Object(UndoObject::Like {
                    id: reaction_id, ..
                }) => {
                    data.undo_reaction(&id, actor.inner(), &reaction_id, Some("Like"))
                        .await
                }
                UndoTarget::Object(UndoObject::Announce {
                    id: reaction_id, ..
                }) => {
                    data.undo_reaction(&id, actor.inner(), &reaction_id, Some("Announce"))
                        .await
                }
                UndoTarget::Id(reaction_id) => {
                    data.undo_reaction(&id, actor.inner(), &reaction_id, None)
                        .await
                }
            },
            Self::Known(KnownActivity::Like { actor, object, id }) => {
                data.record_reaction(&id, actor.inner(), &object, "Like")
                    .await
            }
            Self::Known(KnownActivity::Announce { actor, object, id }) => {
                data.record_reaction(&id, actor.inner(), &object, "Announce")
                    .await
            }
            Self::Known(KnownActivity::Create { actor, object, id }) => {
                if note_is_public(&object) {
                    data.record_reply(&id, actor.inner(), &object).await
                } else {
                    data.record_unsupported(&id, actor.inner(), "Create").await
                }
            }
            Self::Known(KnownActivity::Update { actor, object, id }) => {
                if note_is_public(&object) {
                    data.update_reply(&id, actor.inner(), &object).await
                } else {
                    data.delete_reply(&id, actor.inner(), &object.id, "Update")
                        .await
                }
            }
            Self::Known(KnownActivity::Delete { actor, object, id }) => {
                data.delete_reply(&id, actor.inner(), object.id(), "Delete")
                    .await
            }
            Self::Unsupported(activity) => {
                data.record_unsupported(&activity.id, activity.actor.inner(), &activity.kind)
                    .await
            }
        }
    }
}

async fn verify_remote_note(
    actor: &Url,
    note: &RemoteNote,
    data: &Data<FediverseRepository>,
    require_updated: bool,
) -> Result<(), Error> {
    if note.kind != "Note" {
        return Err(anyhow::anyhow!("unsupported reply object type {}", note.kind).into());
    }
    if actor != note.attributed_to.inner() {
        return Err(anyhow::anyhow!("activity actor does not own reply").into());
    }
    verify_domains_match(&note.id, actor)?;
    if data.reply_post_slug(&note.in_reply_to).await?.is_none() {
        return Err(anyhow::anyhow!("reply does not target a published local post").into());
    }
    if require_updated && note.updated.is_none() {
        return Err(anyhow::anyhow!("reply Update has no updated timestamp").into());
    }
    Ok(())
}

fn note_is_public(note: &RemoteNote) -> bool {
    const PUBLIC: &str = "https://www.w3.org/ns/activitystreams#Public";
    note.to
        .iter()
        .chain(&note.cc)
        .any(|url| url.as_str() == PUBLIC)
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
    replies: CollectionReference,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CollectionReference {
    #[serde(rename = "type")]
    kind: &'static str,
    id: Url,
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
        url: id.clone(),
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
        replies: CollectionReference {
            kind: "Collection",
            id: actor_subresource_url(&id, "replies")?,
        },
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

pub async fn refresh_reply_author_profiles(
    repository: FediverseRepository,
    config: FederationConfig<FediverseRepository>,
) {
    let data = config.to_request_data();
    let actor_ids = match repository.reply_actor_ids().await {
        Ok(actor_ids) => actor_ids,
        Err(error) => {
            tracing::warn!(%error, "Failed to load Fediverse reply authors");
            return;
        }
    };
    for actor_id in actor_ids {
        let actor: ObjectId<FediverseActor> = actor_id.into();
        if let Err(error) = actor.dereference(&data).await {
            tracing::warn!(%error, actor = %actor.inner(), "Failed to refresh Fediverse reply author");
        }
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
        .route("/{user}/posts/{slug}/replies", get(get_post_replies))
        .layer(FederationMiddleware::new(config))
}

struct VerifiedActivityData(ActivityData);

impl<S> FromRequest<S> for VerifiedActivityData
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        const MAX_INBOX_BYTES: usize = 256 * 1024;

        let (parts, body) = request.into_parts();
        let bytes = to_bytes(body, MAX_INBOX_BYTES)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE.into_response())?;
        verify_digest(parts.headers.get("digest"), &bytes)
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
        let request = Request::from_parts(parts, Body::from(bytes));
        ActivityData::from_request(request, state).await.map(Self)
    }
}

fn verify_digest(header: Option<&axum::http::HeaderValue>, body: &[u8]) -> Result<(), Error> {
    let value = header
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("missing or invalid Digest header"))?;
    let encoded = value
        .split(',')
        .find_map(|part| {
            let (algorithm, digest) = part.trim().split_once('=')?;
            algorithm
                .eq_ignore_ascii_case("sha-256")
                .then_some(digest.trim())
        })
        .ok_or_else(|| anyhow::anyhow!("Digest header has no SHA-256 value"))?;
    let expected = BASE64.decode(encoded)?;
    if expected.as_slice() != Sha256::digest(body).as_slice() {
        return Err(anyhow::anyhow!("activity body digest does not match").into());
    }
    Ok(())
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

async fn post_inbox(
    Path(name): Path<String>,
    data: Data<FediverseRepository>,
    VerifiedActivityData(activity_data): VerifiedActivityData,
) -> Response {
    if name != data.username {
        return StatusCode::NOT_FOUND.into_response();
    }
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

async fn get_post_replies(
    Path((name, slug)): Path<(String, String)>,
    data: Data<FediverseRepository>,
) -> Result<FederationJson<WithContext<FollowersCollection>>, StatusCode> {
    if name != data.username {
        return Err(StatusCode::NOT_FOUND);
    }
    let actor = data.local_actor().await.map_err(internal_error)?;
    let root = actor_post_url(actor.ap_id.inner(), &slug).map_err(internal_error)?;
    if data
        .published_post_slug(&root)
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let replies = data
        .direct_reply_ids(&slug, &root)
        .await
        .map_err(internal_error)?;
    Ok(FederationJson(WithContext::new_default(
        FollowersCollection {
            kind: "OrderedCollection",
            id: actor_subresource_url(&root, "replies").map_err(internal_error)?,
            total_items: replies.len(),
            ordered_items: replies,
        },
    )))
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
            display_name: None,
            profile_url: None,
            avatar_url: None,
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

    #[test]
    fn digest_must_match_activity_body() {
        let body = br#"{"type":"Like"}"#;
        let digest = BASE64.encode(Sha256::digest(body));
        let header = axum::http::HeaderValue::from_str(&format!("SHA-256={digest}")).unwrap();

        assert!(verify_digest(Some(&header), body).is_ok());
        assert!(verify_digest(Some(&header), b"changed").is_err());
        assert!(verify_digest(None, body).is_err());
    }

    #[test]
    fn known_interactions_are_not_parsed_as_unsupported() {
        let activity: AcceptedActivity = serde_json::from_value(serde_json::json!({
            "type": "Like",
            "id": "https://social.example/activities/1",
            "actor": "https://social.example/users/alice",
            "object": "https://fedi.flakm.com/blog/posts/example"
        }))
        .unwrap();

        assert!(matches!(
            activity,
            AcceptedActivity::Known(KnownActivity::Like { .. })
        ));
    }

    #[test]
    fn unknown_interactions_can_be_authenticated_and_ignored() {
        let activity: AcceptedActivity = serde_json::from_value(serde_json::json!({
            "type": "EmojiReact",
            "id": "https://social.example/activities/1",
            "actor": "https://social.example/users/alice",
            "object": "https://fedi.flakm.com/blog/posts/example"
        }))
        .unwrap();

        assert!(matches!(activity, AcceptedActivity::Unsupported(_)));
    }

    #[test]
    fn malformed_known_interactions_can_be_authenticated_and_ignored() {
        let activity: AcceptedActivity = serde_json::from_value(serde_json::json!({
            "type": "Update",
            "id": "https://social.example/activities/1",
            "actor": "https://social.example/users/alice",
            "object": "https://social.example/users/alice"
        }))
        .unwrap();

        assert!(matches!(activity, AcceptedActivity::Unsupported(_)));
    }

    #[test]
    fn only_public_replies_are_publishable() {
        let note = |audience: &str| {
            serde_json::from_value::<RemoteNote>(serde_json::json!({
                "type": "Note",
                "id": "https://social.example/users/alice/statuses/1",
                "attributedTo": "https://social.example/users/alice",
                "inReplyTo": "https://fedi.flakm.com/blog/posts/example",
                "to": audience,
                "content": "A reply"
            }))
            .unwrap()
        };

        assert!(note_is_public(&note(
            "https://www.w3.org/ns/activitystreams#Public"
        )));
        assert!(!note_is_public(&note("https://fedi.flakm.com/blog")));
    }
}
