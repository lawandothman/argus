//! `GET /api/traces` (list) and `GET /api/trace/{id}` (detail + correlated logs).

use std::sync::atomic::Ordering;

use argus_core::{SpanStatus, TraceId};
use argus_store::Storage;
use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

#[derive(Deserialize)]
pub(super) struct ListParams {
    from: Option<u64>,
    to: Option<u64>,
    limit: Option<usize>,
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
    let limit = params.limit.unwrap_or(50);

    let engine = state.engine.read().await;
    let mut roots = engine.root_spans_in_range(window);
    roots.sort_by_key(|span| std::cmp::Reverse(span.duration_nanos()));
    roots.truncate(limit);

    let traces: Vec<JsonValue> = roots
        .iter()
        .map(|root| {
            json!({
                "trace_id": root.trace_id.to_hex(),
                "operation": root.name,
                "service": root.resource.service_name().unwrap_or("?"),
                "duration_ms": root.duration_nanos() as f64 / 1_000_000.0,
                "start_ns": root.start.as_unix_nanos(),
                "failed": root.status == SpanStatus::Error,
            })
        })
        .collect();
    Json(json!({ "traces": traces }))
}

pub(super) async fn detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<JsonValue> {
    let Some(trace_id) = TraceId::from_hex(&id) else {
        return Json(json!({ "error": "invalid trace id" }));
    };

    let engine = state.engine.read().await;
    let spans = engine.trace(&trace_id);
    if spans.is_empty() {
        return Json(json!({ "error": "trace not found" }));
    }
    let logs = engine.logs_for_trace(&trace_id);

    Json(json!({
        "trace_id": id,
        "spans": spans.iter().map(super::span_to_json).collect::<Vec<_>>(),
        "logs": logs.iter().map(super::log_to_json).collect::<Vec<_>>(),
    }))
}
