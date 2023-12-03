use axum::{
    http::{header::CACHE_CONTROL, HeaderValue},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router, body::Body,
};
use http::Request;
use std::path::PathBuf;

use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let parrent = std::env::var("RUNTIME_DIRECTORY").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::Path::new(&parrent).join("assets");

    tracing::info!("Serving files from {:?}", dir);
    for dir in std::fs::read_dir(dir.clone()).unwrap() {
        let dir = dir.unwrap();
        let path = dir.path();
        tracing::info!("path: {:?}", path);
    }

    // build our application with a route
    let app = using_serve_dir_with_assets_fallback(dir);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::info!("listening on {}", listener.local_addr().unwrap());


    axum::serve(listener, app.layer(TraceLayer::new_for_http()).into_make_service()).await.unwrap();
}

async fn my_middleware(request: Request<Body>, next: Next) -> Response {
    // do something with `request`...

    let path = request.uri().path().to_lowercase();
    let mut response = next.run(request).await;

    let should_add_no_cache_header = path.contains("heap");
    if should_add_no_cache_header {
        tracing::warn!("appending headers!: {}", path);
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response
    } else {
        response
    }
}

fn using_serve_dir_with_assets_fallback(dir: PathBuf) -> Router {
    // `ServeDir` allows setting a fallback if an asset is not found
    // so with this `GET /assets/doesnt-exist.jpg` will return `index.html`
    // rather than a 404
    let serve_dir = ServeDir::new(&dir).not_found_service(ServeFile::new(dir.join("index.html")));

    Router::new()
        .route("/foo", get(|| async { "Hi from /foo" }))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(my_middleware))
}
