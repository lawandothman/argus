//! `GET /api/query?q=&at=` — run a PromQL-lite query, return result + plan.

use std::sync::atomic::Ordering;

use argus_core::Timestamp;
use argus_query::Value;
use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::state::AppState;

#[derive(Deserialize)]
pub(super) struct QueryParams {
    q: String,
    /// Evaluation timestamp (unix nanos); defaults to the latest ingested sample.
    at: Option<u64>,
}

pub(super) async fn query(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Json<JsonValue> {
    let eval = Timestamp::from_unix_nanos(
        params
            .at
            .unwrap_or_else(|| state.latest_ns.load(Ordering::Relaxed)),
    );
    let plan = argus_query::plan(&params.q)
        .map(|plan| plan.to_string())
        .ok();
    let engine = state.engine.read().await;
    match argus_query::run(&params.q, &*engine, eval) {
        Ok(value) => {
            Json(json!({ "query": params.q, "plan": plan, "result": value_to_json(&value) }))
        }
        Err(error) => Json(json!({ "query": params.q, "error": error.to_string() })),
    }
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Scalar(scalar) => json!({ "type": "scalar", "value": scalar }),
        Value::Vector(samples) => json!({
            "type": "vector",
            "samples": samples
                .iter()
                .map(|sample| json!({ "labels": sample.labels, "value": sample.value }))
                .collect::<Vec<_>>(),
        }),
    }
}
