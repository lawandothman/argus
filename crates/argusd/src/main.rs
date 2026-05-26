//! `argusd` — the Argus server. Wires the storage, query, and (soon) anomaly
//! engines behind an HTTP API. Run with `--demo` to feed it synthetic
//! telemetry.

mod api;
mod event;
mod feed;
mod state;

use std::time::Duration;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    let demo = std::env::args().any(|arg| arg == "--demo");
    let port: u16 = std::env::var("ARGUS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    let state = AppState::new();
    if demo {
        feed::start(state.clone(), 2_000, Duration::from_millis(250)).await;
        println!("argusd: seeded demo telemetry; live feed running");
    }

    let app = api::router(state).layer(tower_http::cors::CorsLayer::permissive());

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind listener");
    println!("argusd listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
