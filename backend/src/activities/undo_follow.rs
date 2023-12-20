use crate::{
    activities::accept::Accept, database::Database, generate_object_id, objects::person::DbUser,
};
use activitypub_federation::traits::Actor;
use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::FollowType, traits::ActivityHandler,
};
use activitystreams_kinds::activity::UndoType;
use serde::{Deserialize, Serialize};
use url::Url;

use super::follow::Follow;


//  {"@context":"https://www.w3.org/ns/activitystreams","id":"https://hachyderm.io/users/flakm#follows/3466630/undo",
//  "type":"Undo","actor":"https://hachyderm.io/users/flakm",
//  "object":{"id":"https://hachyderm.io/d6e9a487-9a5d-45f0-846b-bf2d90947e38","type":"Follow","actor":"https://hachyderm.io/users/flakm","object":"https://fedi.flakm.com/blog_test2"}}
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Unfollow {
    pub(crate) actor: ObjectId<DbUser>,
    pub(crate) object: Follow,
    #[serde(rename = "type")]
    kind: UndoType,
    id: Url,
}


#[async_trait::async_trait]
impl ActivityHandler for Unfollow {
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

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        tracing::info!("Received unfollow from {}", self.actor.inner());

        let local_user = data.local_user().await?;
        let follower = self.actor.dereference(data).await?;

        // add to followers
        data.remove_follower(&local_user, &follower).await?;

        // send back an accept
        let id = generate_object_id(data.domain())?;
        let accept = Accept::new(local_user.ap_id.clone(), self.object, id.clone());
        tracing::info!("Sending unfollow accept to {}", follower.ap_id);
        local_user
            .send(accept, vec![follower.shared_inbox_or_inbox()], data)
            .await?;
        Ok(())
    }
}
