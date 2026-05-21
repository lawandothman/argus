//! Structured log records.

use serde::{Deserialize, Serialize};

use crate::{
    attributes::Attributes,
    resource::Resource,
    timestamp::Timestamp,
    trace::{SpanId, TraceId},
};

/// Log severity, ordered least-to-most severe so that comparisons such as
/// `severity >= Severity::Warn` are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// A single structured log record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: Timestamp,
    pub severity: Severity,
    pub body: String,
    pub attributes: Attributes,
    pub resource: Resource,

    /// Correlation keys. When present, this record was emitted within the
    /// context of a span and can be joined directly to traces — the backbone of
    /// cross-signal correlation in Argus.
    pub trace_id: Option<TraceId>,
    pub span_id: Option<SpanId>,
}

impl LogRecord {
    /// Whether this record carries trace context for correlation.
    pub fn is_correlated(&self) -> bool {
        self.trace_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders_least_to_most_severe() {
        assert!(Severity::Trace < Severity::Debug);
        assert!(Severity::Error > Severity::Warn);
        assert!(Severity::Fatal > Severity::Error);
    }

    #[test]
    fn correlation_flag_tracks_trace_id() {
        let mut record = LogRecord {
            timestamp: Timestamp::now(),
            severity: Severity::Error,
            body: "payment declined".to_owned(),
            attributes: Attributes::new(),
            resource: Resource::service("payments"),
            trace_id: None,
            span_id: None,
        };
        assert!(!record.is_correlated());

        record.trace_id = Some(TraceId::from_bytes([7; 16]));
        assert!(record.is_correlated());
    }
}
