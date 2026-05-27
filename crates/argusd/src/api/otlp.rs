//! OTLP/HTTP ingest. `POST /v1/traces` accepts an OTLP trace export — protobuf
//! (`application/x-protobuf`) or OTLP/JSON — decodes it, maps it to Argus spans,
//! and ingests them. Mirrors the OTLP/HTTP spec so real OpenTelemetry SDKs and
//! Collectors can export straight to Argus.

use std::sync::atomic::Ordering;

use argus_core::Signal;
use argus_ingest::otlp;
use argus_store::Storage;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

pub(super) async fn ingest_traces(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    let is_json = content_type.is_some_and(|ct| ct.starts_with("application/json"));

    let request = match otlp::decode_trace_export(content_type, &body) {
        Ok(request) => request,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };

    let spans = otlp::spans_from_export(request);
    let accepted = spans.len();
    let latest = spans.iter().map(|span| span.start.as_unix_nanos()).max();

    state
        .engine
        .write()
        .await
        .ingest_all(spans.into_iter().map(Signal::from));
    if let Some(latest) = latest {
        state.latest_ns.fetch_max(latest, Ordering::Relaxed);
    }
    tracing::info!(accepted, "ingested OTLP trace export");

    // OTLP success is an empty ExportTraceServiceResponse: `{}` / zero bytes.
    if is_json {
        ([(header::CONTENT_TYPE, "application/json")], "{}").into_response()
    } else {
        (
            [(header::CONTENT_TYPE, "application/x-protobuf")],
            Bytes::new(),
        )
            .into_response()
    }
}
