//! `GET /api/stats` — engine contents and compression.

use std::sync::atomic::Ordering;

use argus_store::Storage;
use axum::Json;
use axum::extract::State;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

pub(super) async fn stats(State(state): State<AppState>) -> Json<JsonValue> {
    let stats = state.engine.read().await.stats();
    Json(json!({
        "series": stats.series,
        "samples": stats.samples,
        "spans": stats.spans,
        "traces": stats.traces,
        "logs": stats.logs,
        "metric_bytes": stats.metric_bytes,
        "compression_ratio": stats.compression_ratio(),
        "bytes_per_sample": stats.bytes_per_sample(),
        "latest_ns": state.latest_ns.load(Ordering::Relaxed),
    }))
}
