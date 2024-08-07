use crate::{
    database::Repository,
    error::Error,
    objects::{person::DbUser, post::Note},
    utils::generate_object_id,
    FediPost,
};
use activitypub_federation::{
    activity_sending::SendActivityTask,
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::CreateType,
    protocol::{context::WithContext, helpers::deserialize_one_or_many},
    traits::{ActivityHandler, Object},
};
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreatePost {
    pub(crate) actor: ObjectId<DbUser>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub(crate) to: Vec<Url>,
    pub(crate) object: Note,
    #[serde(rename = "type")]
    pub(crate) kind: CreateType,
    pub(crate) id: Url,
}

impl CreatePost {
    pub fn new(note: Note, id: Url) -> CreatePost {
        CreatePost {
            actor: note.attributed_to.clone(),
            to: note.to.clone(),
            object: note,
            kind: CreateType::Create,
            id,
        }
    }
}

impl CreatePost {
    pub async fn send(note: Note, inbox: Url, data: &Data<Repository>) -> Result<(), Error> {
        info!("Sending reply to {}", &note.attributed_to);
        let create = CreatePost {
            actor: note.attributed_to.clone(),
            to: note.to.clone(),
            object: note,
            kind: CreateType::Create,
            id: generate_object_id(data.domain())?,
        };
        let create_with_context = WithContext::new_default(create);
        let local_user = data.blog_user().await?;
        let sends =
            SendActivityTask::prepare(&create_with_context, &local_user, vec![inbox], data).await?;
        for send in sends {
            send.sign_and_send(data).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActivityHandler for CreatePost {
    type DataType = Repository;
    type Error = crate::error::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        FediPost::verify(&self.object, &self.id, data).await?;
        Ok(())
    }

    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }
}
