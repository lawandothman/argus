//! Events pushed to live (`WS /api/stream`) subscribers.

use serde::Serialize;

/// A single live telemetry event. Tagged by `kind` on the wire so the client
/// can switch on it; more variants (e.g. anomaly events) will join `Request`.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    /// A completed request: its end-to-end latency and outcome.
    Request {
        trace_id: String,
        route: String,
        status: u16,
        duration_ms: f64,
        failed: bool,
        timestamp_ns: u64,
    },
}
