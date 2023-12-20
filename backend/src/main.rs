use crate::http::http_get_user_followers;
use crate::{
    database::Database,
    http::{http_get_user, http_post_to_followers, http_post_user_inbox, webfinger},
    objects::{person::DbUser, post::DbPost},
    utils::generate_object_id,
};
use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use axum::response::Response;
use axum::{
    http::Request,
    routing::{get, post},
    Router,
};
use error::Error;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::net::ToSocketAddrs;
use tracing::log::info;

use axum::extract::MatchedPath;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{field, info_span, Span};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod activities;
mod database;
mod error;
#[allow(clippy::diverging_sub_expression, clippy::items_after_statements)]
mod http;
mod objects;
mod utils;

const DOMAIN: &str = "fedi.flakm.com";
const LOCAL_USER_NAME: &str = "blog_test2";
const BIND_ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "backend=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "./db.sqlite".into());

    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // Run migrations in ./migrations
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Migrations run");

    info!("Setup local user and database");

    let database = Database { pool };

    // local user might not be initialized yet
    let _local_user = match database.read_user(LOCAL_USER_NAME).await? {
        Some(local_user) => {
            info!("Local user already exists");
            local_user // user already exists
        }
        None => {
            let local_user = DbUser::new(DOMAIN, LOCAL_USER_NAME)?;
            database.save_user(&local_user).await?;
            info!("Created local user");
            local_user
        }
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
        .route("/:user/followers", get(http_get_user_followers))
        .route("/.well-known/webfinger", get(webfinger))
        .route("/followers", post(http_post_to_followers))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        path = ?request.uri().path(),
                        matched_path,
                        latency = tracing::field::Empty,
                        status_code = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|response: &Response, latency: Duration, span: &Span| {
                    span.record("latency", field::debug(&latency));
                    span.record("status_code", response.status().as_u16());
                    tracing::info!("response");
                }),
        )
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
