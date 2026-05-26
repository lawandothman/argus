//! `GET /api/logs?from=&to=&severity=&limit=` — recent logs in a window.

use std::sync::atomic::Ordering;

use argus_core::Severity;
use argus_store::Storage;
use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

#[derive(Deserialize)]
pub(super) struct ListParams {
    from: Option<u64>,
    to: Option<u64>,
    limit: Option<usize>,
    severity: Option<String>,
}

pub(super) async fn list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<JsonValue> {
    let window = super::time_window(
        params.from,
        params.to,
        state.latest_ns.load(Ordering::Relaxed),
    );
    let min_severity = params.severity.as_deref().and_then(parse_severity);
    let limit = params.limit.unwrap_or(100);

    let engine = state.engine.read().await;
    let mut logs = engine.logs_in_range(window);
    if let Some(min) = min_severity {
        logs.retain(|log| log.severity >= min);
    }
    logs.sort_by_key(|log| std::cmp::Reverse(log.timestamp.as_unix_nanos()));
    logs.truncate(limit);

    Json(json!({ "logs": logs.iter().map(super::log_to_json).collect::<Vec<_>>() }))
}

fn parse_severity(value: &str) -> Option<Severity> {
    match value.to_ascii_lowercase().as_str() {
        "trace" => Some(Severity::Trace),
        "debug" => Some(Severity::Debug),
        "info" => Some(Severity::Info),
        "warn" => Some(Severity::Warn),
        "error" => Some(Severity::Error),
        "fatal" => Some(Severity::Fatal),
        _ => None,
    }
}
