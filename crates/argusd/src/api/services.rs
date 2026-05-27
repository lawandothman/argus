//! `GET /api/services` — per-service throughput, error rate, and latency
//! percentiles over a recent window, aggregated from stored spans. This is what
//! the service map renders: a real number on every node, not decoration.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use argus_core::{SpanStatus, Timestamp};
use argus_store::{Storage, TimeRange};
use axum::Json;
use axum::extract::State;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

/// How far back to aggregate.
const WINDOW_NS: u64 = 60 * 1_000_000_000;

#[derive(Default)]
struct Aggregate {
    durations_ms: Vec<f64>,
    errors: usize,
}

pub(super) async fn list(State(state): State<AppState>) -> Json<JsonValue> {
    let latest = state.latest_ns.load(Ordering::Relaxed);
    let window = TimeRange::new(
        Timestamp::from_unix_nanos(latest.saturating_sub(WINDOW_NS)),
        Timestamp::from_unix_nanos(latest),
    );
    let window_secs = (WINDOW_NS / 1_000_000_000) as f64;

    let engine = state.engine.read().await;

    // Walk every span of every trace that started in the window, bucketed by the
    // service that emitted it. Each trace is visited once (via its root), so no
    // span is counted twice.
    let mut by_service: HashMap<String, Aggregate> = HashMap::new();
    for root in engine.root_spans_in_range(window) {
        for span in engine.trace(&root.trace_id) {
            let service = span.resource.service_name().unwrap_or("?").to_owned();
            let aggregate = by_service.entry(service).or_default();
            aggregate
                .durations_ms
                .push(span.duration_nanos() as f64 / 1_000_000.0);
            if span.status == SpanStatus::Error {
                aggregate.errors += 1;
            }
        }
    }

    let mut services: Vec<(String, Aggregate)> = by_service.into_iter().collect();
    services.sort_by(|(a, _), (b, _)| a.cmp(b));

    let services: Vec<JsonValue> = services
        .into_iter()
        .map(|(service, mut aggregate)| {
            aggregate.durations_ms.sort_unstable_by(f64::total_cmp);
            let calls = aggregate.durations_ms.len();
            json!({
                "service": service,
                "calls": calls,
                "rps": calls as f64 / window_secs,
                "error_rate": if calls > 0 { aggregate.errors as f64 / calls as f64 } else { 0.0 },
                "p50": percentile(&aggregate.durations_ms, 0.50),
                "p95": percentile(&aggregate.durations_ms, 0.95),
                "p99": percentile(&aggregate.durations_ms, 0.99),
            })
        })
        .collect();

    Json(json!({
        "window_ms": WINDOW_NS / 1_000_000,
        "services": services,
    }))
}

/// Nearest-rank percentile of a pre-sorted slice of milliseconds.
fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (quantile * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}
