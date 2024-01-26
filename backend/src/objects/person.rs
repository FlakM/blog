use crate::{
    activities::{accept::Accept, create_post::CreatePost, follow::Follow, undo_follow::Unfollow},
    database::Repository,
    error::Error,
    utils::generate_object_id,
};

use activitypub_federation::{
    activity_sending::SendActivityTask,
    config::Data,
    fetch::object_id::ObjectId,
    http_signatures::generate_actor_keypair,
    kinds::actor::PersonType,
    protocol::{context::WithContext, public_key::PublicKey, verification::verify_domains_match},
    traits::{ActivityHandler, Actor, Object},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, FromRow, Row};
use std::fmt::Debug;
use url::Url;

use super::post::FediPost;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ImageType {
    Image,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MediaType {
    #[serde(rename = "image/jpeg")]
    Jpg,
}

impl FromRow<'_, SqliteRow> for ImageType {
    fn from_row(row: &SqliteRow) -> std::result::Result<ImageType, sqlx::Error> {
        let value: String = row.try_get("image_type")?;
        match value.as_str() {
            "Image" => Ok(ImageType::Image),
            _ => Err(sqlx::Error::TypeNotFound {
                type_name: "ImageType".to_string(),
            }),
        }
    }
}

impl<'r> FromRow<'r, SqliteRow> for MediaType {
    fn from_row(row: &'r SqliteRow) -> std::result::Result<MediaType, sqlx::Error> {
        let value: String = row.try_get("media_type")?;
        match value.as_str() {
            "image/jpeg" => Ok(MediaType::Jpg),
            _ => Err(sqlx::Error::TypeNotFound {
                type_name: "mediaType".to_string(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    #[serde(rename = "type")]
    pub kind: ImageType,
    pub media_type: MediaType,
    pub url: Url,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbUser {
    pub name: String,
    pub ap_id: ObjectId<DbUser>,
    pub inbox: Url,
    // exists for all users (necessary to verify http signatures)
    pub public_key: String,
    // exists only for local users
    pub private_key: Option<String>,
    last_refreshed_at: DateTime<Utc>,
    pub local: bool,
    pub icon: Option<Icon>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Follower {
    pub follower_url: Url,
}

impl TryFrom<String> for Follower {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Follower {
            follower_url: Url::parse(&value)?,
        })
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct SqliteUser {
    pub id: i64,
    pub name: String,
    pub user: sqlx::types::Json<DbUser>,
    pub object_id: String,
}

impl From<SqliteUser> for DbUser {
    fn from(sqlite_user: SqliteUser) -> Self {
        sqlite_user.user.0
    }
}

/// List of all activities which this actor can receive.
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
#[enum_delegate::implement(ActivityHandler)]
pub enum PersonAcceptedActivities {
    CreateNote(CreatePost),
    Follow(Follow),
    Accept(Accept),
    UndoFollow(Unfollow),
}

impl DbUser {
    pub fn new(hostname: &str, name: &str) -> Result<DbUser, Error> {
        let ap_id = Url::parse(&format!("https://{}/{}", hostname, &name))?.into();
        let inbox = Url::parse(&format!("https://{}/{}/inbox", hostname, &name))?;
        let keypair = generate_actor_keypair()?;
        Ok(DbUser {
            name: name.to_string(),
            ap_id,
            inbox,
            public_key: keypair.public_key,
            private_key: Some(keypair.private_key),
            last_refreshed_at: Utc::now(),
            local: true,
            icon: Some(Icon {
                kind: ImageType::Image,
                media_type: MediaType::Jpg,
                url: Url::parse("https://media.hachyderm.io/accounts/avatars/110/178/726/811/515/304/original/230c44c3d25cf3ba.jpg")?,
            }),
            summary: Some("I'm a bot that posts random images from the internet. I'm not a real person, but I'm still a nice bot.".to_string()),
        })
    }

    pub fn followers_url(&self) -> Result<Url, Error> {
        Ok(Url::parse(&format!("{}/followers", self.ap_id.inner()))?)
    }

    pub async fn post(&self, post: &FediPost, data: &Data<Repository>) -> Result<(), Error> {
        let id = generate_object_id(data.domain())?;
        let create = CreatePost::new(post.clone().into_json(data).await?, id.clone());
        let mut inboxes = vec![];

        for f in data.user_followers(self).await? {
            let user: DbUser = ObjectId::from(f.follower_url).dereference(data).await?;
            let mailbox = user.shared_inbox_or_inbox();
            inboxes.push(mailbox);
        }

        tracing::info!(
            "Sending post to inboxes [{}]",
            inboxes
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.send(create, inboxes, data).await?;
        Ok(())
    }

    pub(crate) async fn send<Activity>(
        &self,
        activity: Activity,
        recipients: Vec<Url>,
        data: &Data<Repository>,
    ) -> Result<(), Error>
    where
        Activity: ActivityHandler + Serialize + Debug + Send + Sync,
        <Activity as ActivityHandler>::Error: From<anyhow::Error> + From<serde_json::Error>,
    {
        let activity = WithContext::new_default(activity);
        let sends = SendActivityTask::prepare(&activity, self, recipients, data).await?;
        for send in sends {
            send.sign_and_send(data).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "type")]
    kind: PersonType,
    preferred_username: String,
    id: ObjectId<DbUser>,
    inbox: Url,
    public_key: PublicKey,
    icon: Option<Icon>,
    summary: Option<String>,
}

#[async_trait::async_trait]
impl Object for DbUser {
    type DataType = Repository;
    type Kind = Person;
    type Error = Error;

    fn last_refreshed_at(&self) -> Option<DateTime<Utc>> {
        Some(self.last_refreshed_at)
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        data.user_by_object_id(object_id.as_str()).await
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(Person {
            preferred_username: self.name.clone(),
            kind: Default::default(),
            id: self.ap_id.clone(),
            inbox: self.inbox.clone(),
            public_key: self.public_key(),
            icon: self.icon,
            summary: self.summary,
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(
        json: Self::Kind,
        _data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        Ok(DbUser {
            name: json.preferred_username,
            ap_id: json.id,
            inbox: json.inbox,
            public_key: json.public_key.public_key_pem,
            private_key: None,
            last_refreshed_at: Utc::now(),
            local: false,
            icon: None,
            summary: None,
        })
    }
}

impl Actor for DbUser {
    fn id(&self) -> Url {
        self.ap_id.inner().clone()
    }

    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone()
    }

    fn inbox(&self) -> Url {
        self.inbox.clone()
    }
}
