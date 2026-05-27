//! HTTP API: route table plus shared response helpers.

mod anomalies;
mod logs;
mod query;
mod services;
mod stats;
mod stream;
mod traces;

use argus_core::{LogRecord, Span, Timestamp};
use argus_store::TimeRange;
use axum::Router;
use axum::routing::get;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/stats", get(stats::stats))
        .route("/api/query", get(query::query))
        .route("/api/traces", get(traces::list))
        .route("/api/trace/{id}", get(traces::detail))
        .route("/api/services", get(services::list))
        .route("/api/logs", get(logs::list))
        .route("/api/anomalies", get(anomalies::detect))
        .route("/api/stream", get(stream::stream))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

/// Resolve a `from`/`to` (unix-nanos) query window, defaulting to the last five
/// minutes ending at the latest ingested sample.
fn time_window(from: Option<u64>, to: Option<u64>, latest_ns: u64) -> TimeRange {
    const FIVE_MINUTES_NS: u64 = 5 * 60 * 1_000_000_000;
    let end = to.unwrap_or(latest_ns);
    let start = from.unwrap_or_else(|| end.saturating_sub(FIVE_MINUTES_NS));
    TimeRange::new(
        Timestamp::from_unix_nanos(start),
        Timestamp::from_unix_nanos(end),
    )
}

fn span_to_json(span: &Span) -> JsonValue {
    json!({
        "span_id": span.span_id.to_hex(),
        "parent_span_id": span.parent_span_id.map(|id| id.to_hex()),
        "name": span.name,
        "service": span.resource.service_name().unwrap_or("?"),
        "kind": format!("{:?}", span.kind),
        "status": format!("{:?}", span.status),
        "start_ns": span.start.as_unix_nanos(),
        "duration_ms": span.duration_nanos() as f64 / 1_000_000.0,
        "attributes": serde_json::to_value(&span.attributes).unwrap_or_default(),
    })
}

fn log_to_json(log: &LogRecord) -> JsonValue {
    json!({
        "timestamp_ns": log.timestamp.as_unix_nanos(),
        "severity": format!("{:?}", log.severity),
        "body": log.body,
        "service": log.resource.service_name().unwrap_or("?"),
        "trace_id": log.trace_id.map(|id| id.to_hex()),
        "span_id": log.span_id.map(|id| id.to_hex()),
    })
}
