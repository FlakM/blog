use crate::{
    database::Database,
    error::Error,
    objects::{
        person::{DbUser, Person, PersonAcceptedActivities},
        post::DbPost,
    },
};
use activitypub_federation::{
    axum::{
        inbox::{receive_activity, ActivityData},
        json::FederationJson,
    },
    config::Data,
    fetch::webfinger::{build_webfinger_response, extract_webfinger_name, Webfinger},
    protocol::context::WithContext,
    traits::Object,
};
use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Response},
    Json,
};
use axum_macros::debug_handler;
use http::StatusCode;
use serde::Deserialize;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self.0)).into_response()
    }
}

#[debug_handler]
pub async fn http_get_user(
    Path(name): Path<String>,
    data: Data<Database>,
) -> Result<FederationJson<WithContext<Person>>, Error> {
    let db_user = data.read_user(&name).await?;
    let json_user = db_user
        .ok_or(Error(anyhow::anyhow!("User not found")))?
        .into_json(&data)
        .await?;
    Ok(FederationJson(WithContext::new_default(json_user)))
}

#[debug_handler]
pub async fn http_post_user_inbox(
    data: Data<Database>,
    activity_data: ActivityData,
) -> impl IntoResponse {
    receive_activity::<WithContext<PersonAcceptedActivities>, DbUser, Database>(
        activity_data,
        &data,
    )
    .await
}

#[derive(Deserialize)]
pub struct WebfingerQuery {
    resource: String,
}

#[debug_handler]
pub async fn webfinger(
    Query(query): Query<WebfingerQuery>,
    data: Data<Database>,
) -> Result<Json<Webfinger>, Error> {
    let name = extract_webfinger_name(&query.resource, &data)?;
    let db_user = data
        .read_user(name)
        .await?
        .ok_or(Error(anyhow::anyhow!("Resource not found")))?;
    Ok(Json(build_webfinger_response(
        query.resource,
        db_user.ap_id.into_inner(),
    )))
}

#[debug_handler]
pub async fn http_post_to_followers(data: Data<Database>) -> Result<impl IntoResponse, Error> {
    let local_user = data.local_user().await?;
    let post = DbPost {
        text: "<p>Hello world @flakm</p>".to_string(),
        ap_id: "https://fedi.flakm.com/123".parse()?,
        creator: local_user.ap_id.clone(),
        local: false,
    };

    local_user.post(post, &data).await?;

    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn http_get_user_followers(
    Path(name): Path<String>,
    data: Data<Database>,
) -> Result<impl IntoResponse, Error> {
    let user = data
        .read_user(&name)
        .await?
        .ok_or(Error(anyhow::anyhow!("User not found")))?;
    let followers = data.get_followers(&user).await?;
    Ok(Json(followers))
}
