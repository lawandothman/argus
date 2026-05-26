//! `GET /api/anomalies?metric=&status=` — run the detectors over a metric series
//! and correlate the first changepoint to its likely cause.

use argus_anomaly::{Cusum, Detector, Ewma, Mad, explain};
use argus_core::Timestamp;
use argus_store::{Matcher, Selector, Storage, TimeRange};
use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

#[derive(Deserialize)]
pub(super) struct Params {
    metric: Option<String>,
    status: Option<String>,
}

pub(super) async fn detect(
    State(state): State<AppState>,
    Query(params): Query<Params>,
) -> Json<JsonValue> {
    let metric = params
        .metric
        .unwrap_or_else(|| "http_request_duration_ms".to_owned());
    let status = params.status.unwrap_or_else(|| "200".to_owned());

    let engine = state.engine.read().await;
    let selector = Selector::new(&metric).with(Matcher::eq("status", &status));
    let series: Vec<(u64, f64)> = engine
        .query_metrics(&selector, TimeRange::all())
        .iter()
        .flat_map(|result| {
            result
                .samples
                .iter()
                .map(|sample| (sample.timestamp.as_unix_millis(), sample.value))
        })
        .collect();

    let ewma = Ewma::default().detect(&series);
    let mad = Mad::default().detect(&series);
    let cusum = Cusum::default().detect(&series);

    let changepoint = cusum.first();
    let cause = changepoint.map(|point| {
        let start = Timestamp::from_unix_millis(point.timestamp_ms.saturating_sub(200));
        let end = Timestamp::from_unix_millis(point.timestamp_ms + 1_500);
        explain(&*engine, TimeRange::new(start, end), 5)
    });

    Json(json!({
        "metric": metric,
        "status": status,
        "points": series.len(),
        "detectors": { "ewma": ewma.len(), "mad": mad.len(), "cusum": cusum.len() },
        "changepoint": changepoint.map(|point| json!({
            "timestamp_ms": point.timestamp_ms,
            "observed": point.observed,
            "expected": point.expected,
            "score": point.score,
        })),
        "cause": cause.map(|explanation| json!({
            "slowest": explanation
                .slowest
                .iter()
                .map(|trace| json!({
                    "trace_id": trace.trace_id.to_hex(),
                    "operation": trace.root_op,
                    "duration_ms": trace.duration_ms,
                    "failed": trace.failed,
                    "bottleneck_service": trace.slowest_service,
                    "bottleneck_op": trace.slowest_op,
                    "bottleneck_ms": trace.slowest_ms,
                }))
                .collect::<Vec<_>>(),
            "error_logs": explanation.error_logs.iter().map(super::log_to_json).collect::<Vec<_>>(),
        })),
    }))
}
