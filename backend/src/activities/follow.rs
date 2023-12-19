use crate::{
    activities::accept::Accept, database::Database, generate_object_id, objects::person::DbUser,
};
use activitypub_federation::traits::Actor;
use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::FollowType, traits::ActivityHandler,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Follow {
    pub(crate) actor: ObjectId<DbUser>,
    pub(crate) object: ObjectId<DbUser>,
    #[serde(rename = "type")]
    kind: FollowType,
    id: Url,
}

impl Follow {
    pub fn new(actor: ObjectId<DbUser>, object: ObjectId<DbUser>, id: Url) -> Follow {
        Follow {
            actor,
            object,
            kind: Default::default(),
            id,
        }
    }
}

#[async_trait::async_trait]
impl ActivityHandler for Follow {
    type DataType = Database;
    type Error = crate::error::Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    // Ignore clippy false positive: https://github.com/rust-lang/rust-clippy/issues/6446
    #[allow(clippy::await_holding_lock)]
    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        tracing::info!("Received follow from {}", self.actor.inner());

        let local_user = data.local_user().await?;
        let follower = self.actor.dereference(data).await?;

        // add to followers
        data.save_follower(&local_user, &follower).await?;

        // send back an accept
        let id = generate_object_id(data.domain())?;
        let accept = Accept::new(local_user.ap_id.clone(), self, id.clone());
        tracing::info!("Sending accept to {}", follower.ap_id);
        local_user
            .send(accept, vec![follower.shared_inbox_or_inbox()], data)
            .await?;
        Ok(())
    }
}
