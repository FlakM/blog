use crate::{
    database::Database,
    http::{http_get_user, http_post_to_followers, http_post_user_inbox, webfinger},
    objects::{
        person::{DbUser, SqliteUser},
        post::DbPost,
    },
    utils::generate_object_id,
};
use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use axum::{
    routing::{get, post},
    Router,
};
use error::Error;
use sqlx::sqlite::SqlitePoolOptions;
use std::{
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
};
use tracing::log::info;

mod activities;
mod database;
mod error;
#[allow(clippy::diverging_sub_expression, clippy::items_after_statements)]
mod http;
mod objects;
mod utils;

const DOMAIN: &str = "fedi.flakm.com";
const LOCAL_USER_NAME: &str = "blog_test";
const BIND_ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::builder().format_timestamp(None).init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./db.sqlite".into());
    let pool = SqlitePoolOptions::new().connect(&database_url).await?;

    // Run migrations in ./migrations
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Migrations run");

    info!("Setup local user and database");

    let database = Database {
        pool: pool.clone(),
    };

    let local_user = database.read_user(LOCAL_USER_NAME).await?;


    let local_user = match local_user {
        Some(local_user) => local_user, // user already exists
        None => {
            let local_user = DbUser::new(DOMAIN, LOCAL_USER_NAME)?;

            // serialize to string and print to stdout
            let local_user_json = serde_json::to_string(&local_user)?;
            println!("{}", local_user_json);

            database.save_user(&local_user).await?;
            local_user
        }
    };

    let database = Database {
        pool
    };

    info!("Setup configuration");
    let config = FederationConfig::builder()
        .domain(DOMAIN)
        .app_data(database)
        .build()
        .await?;

    info!("Listen with HTTP server on {BIND_ADDRESS}");
    let config = config.clone();
    let app = Router::new()
        .route("/:user", get(http_get_user))
        .route("/:user/inbox", post(http_post_user_inbox))
        .route("/.well-known/webfinger", get(webfinger))
        .route("/followers", post(http_post_to_followers))
        .layer(FederationMiddleware::new(config));

    let addr = BIND_ADDRESS
        .to_socket_addrs()?
        .next()
        .expect("Failed to lookup domain name");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
