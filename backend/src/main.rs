use crate::database::Repository;
use crate::http::http_get_user_followers;
use crate::hugo_posts::HugoBlogPost;
use crate::{
    database::SqlDatabase,
    http::{http_get_user, http_post_user_inbox, webfinger},
    objects::{person::DbUser, post::FediPost},
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
use std::fs;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::MatchedPath;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{field, info, info_span, Span};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod activities;
mod database;
mod error;
#[allow(clippy::diverging_sub_expression, clippy::items_after_statements)]
mod http;
mod hugo_posts;
mod objects;
mod utils;

const LOCAL_USER_NAME: &str = "blog";
const BIND_ADDRESS: &str = "127.0.0.1:3000";
const DOMAIN: &str = "fedi.flakm.com";

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
    let db_path: PathBuf = database_path.clone().into();

    // 2) make sure its parent directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::from(e))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // Run migrations in ./migrations
    sqlx::migrate!().run(&pool).await?;
    info!("Migrations run");

    let posts_path = std::env::args().nth(1).expect("No posts file given");
    let blog_repo = hugo_posts::BlogRepository { db: pool.clone() };
    let blog_posts = HugoBlogPost::load_new_posts(posts_path).expect("Failed to load blog posts");

    for blog_post in blog_posts {
        info!("Processing: {}", blog_post.slug);
        blog_repo.new_blog_entry(&blog_post).await?;
    }

    info!("Setup local user and database");

    let database = Arc::new(SqlDatabase { pool }) as Repository;

    // local user might not be initialized yet
    let local_user = match database.user_by_name(LOCAL_USER_NAME).await? {
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

    let data = config.to_request_data();
    let mut to_be_published = vec![];
    for post in blog_repo.get_unpublished_blog_posts().await? {
        let post = post.into_post(&local_user, data.domain())?;
        to_be_published.push(post);
    }

    let publish = async move {
        // wait 5 seconds before publishing for the server to start
        tokio::time::sleep(Duration::from_secs(5)).await;
        tracing::info!("Publishing posts...");
        to_be_published.sort_by(|a, b| a.blog_post.date.cmp(&b.blog_post.date));
        for post in to_be_published {
            tracing::info!("Publishing post: {}", post.blog_post.slug);
            local_user.post(&post, &data).await.unwrap();
            blog_repo.mark_as_published(&post.blog_post).await.unwrap();
        }
        Ok::<_, Error>(())
    };

    info!("Listen with HTTP server on {BIND_ADDRESS}");
    let config = config.clone();
    let app = Router::new()
        .route("/:user", get(http_get_user))
        .route("/:user/inbox", post(http_post_user_inbox))
        .route("/:user/followers", get(http_get_user_followers))
        .route("/.well-known/webfinger", get(webfinger))
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
                    info!("response");
                }),
        )
        .layer(FederationMiddleware::new(config));

    let addr = BIND_ADDRESS
        .to_socket_addrs()?
        .next()
        .expect("Failed to lookup domain name");
    let server = async {
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .map_err(Error::from)
    };

    // use try_join to run the server and the publish task concurrently
    let res = tokio::try_join!(server, publish);

    match res {
        Ok((_first, _second)) => {
            // do something with the values
        }
        Err(err) => {
            panic!("processing failed; error = {}", err);
        }
    };

    Ok(())
}
