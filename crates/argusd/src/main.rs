//! `argusd` — the Argus server. Wires the storage, query, and anomaly engines
//! behind an HTTP + WebSocket API. Run with `--demo` to feed it synthetic
//! telemetry.

mod api;
mod event;
mod feed;
mod state;

use std::time::Duration;

use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    // An observability platform ought to be observable: structured, leveled
    // logs and a span per request (override the default with RUST_LOG).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let demo = std::env::args().any(|arg| arg == "--demo");
    let port: u16 = std::env::var("ARGUS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    let state = AppState::new();
    if demo {
        feed::start(state.clone(), 2_000, Duration::from_millis(250)).await;
        info!("seeded demo telemetry; live feed running");
    }

    let app = api::router(state).layer(CorsLayer::permissive()).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind listener");
    info!("listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
