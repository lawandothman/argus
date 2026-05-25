//! The in-memory engine: composes the metric, trace, and log stores behind the
//! [`Storage`] trait.

use argus_core::{LogRecord, Signal, Span, TraceId};

use crate::logs::LogStore;
use crate::metrics::MetricStore;
use crate::query::{Selector, SeriesResult, TimeRange};
use crate::stats::StorageStats;
use crate::storage::Storage;
use crate::traces::TraceStore;

/// An all-in-memory implementation of [`Storage`].
#[derive(Debug, Default)]
pub struct MemoryEngine {
    metrics: MetricStore,
    traces: TraceStore,
    logs: LogStore,
}

impl MemoryEngine {
    pub fn new() -> Self {
        MemoryEngine::default()
    }
}

impl Storage for MemoryEngine {
    fn ingest(&mut self, signal: Signal) {
        match signal {
            Signal::Metric(point) => self.metrics.append(point),
            Signal::Span(span) => self.traces.insert(span),
            Signal::Log(record) => self.logs.insert(record),
        }
    }

    fn query_metrics(&self, selector: &Selector, range: TimeRange) -> Vec<SeriesResult> {
        self.metrics.query(selector, range)
    }

    fn trace(&self, id: &TraceId) -> Vec<Span> {
        self.traces.trace(id)
    }

    fn logs_for_trace(&self, id: &TraceId) -> Vec<LogRecord> {
        self.logs.for_trace(id)
    }

    fn stats(&self) -> StorageStats {
        StorageStats {
            series: self.metrics.series_count(),
            samples: self.metrics.sample_count(),
            spans: self.traces.span_count(),
            traces: self.traces.trace_count(),
            logs: self.logs.len(),
            metric_bytes: self.metrics.encoded_len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argus_core::{
        Attributes, Labels, LogRecord, MetricKind, MetricPoint, Resource, Sample, Severity, Span,
        SpanId, SpanKind, SpanStatus, Timestamp, TraceId,
    };

    use crate::query::Matcher;

    fn point(route: &str, status: &str, ts: u64, value: f64) -> MetricPoint {
        MetricPoint::new(
            "http_request_duration_ms",
            MetricKind::Gauge,
            Labels::new().with("route", route).with("status", status),
            Sample::new(Timestamp::from_unix_millis(ts), value),
        )
    }

    #[test]
    fn metric_query_matches_labels_and_range() {
        let mut engine = MemoryEngine::new();
        engine.ingest(point("/checkout", "200", 1_000, 100.0).into());
        engine.ingest(point("/checkout", "200", 2_000, 120.0).into());
        engine.ingest(point("/checkout", "500", 1_500, 900.0).into());

        let ok = engine.query_metrics(
            &Selector::new("http_request_duration_ms").with(Matcher::eq("status", "200")),
            TimeRange::all(),
        );
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].samples.len(), 2);

        // a narrow range trims samples
        let narrow = engine.query_metrics(
            &Selector::new("http_request_duration_ms").with(Matcher::eq("status", "200")),
            TimeRange::new(
                Timestamp::from_unix_millis(1_500),
                Timestamp::from_unix_millis(2_500),
            ),
        );
        assert_eq!(narrow[0].samples.len(), 1);
    }

    #[test]
    fn trace_and_logs_correlate() {
        let mut engine = MemoryEngine::new();
        let trace_id = TraceId::from_bytes([9; 16]);
        let span_id = SpanId::from_bytes([1; 8]);

        engine.ingest(
            Span {
                trace_id,
                span_id,
                parent_span_id: None,
                name: "GET /checkout".to_owned(),
                kind: SpanKind::Server,
                start: Timestamp::from_unix_nanos(1_000),
                end: Timestamp::from_unix_nanos(5_000),
                status: SpanStatus::Ok,
                attributes: Attributes::new(),
                resource: Resource::service("api-gateway"),
            }
            .into(),
        );
        engine.ingest(
            LogRecord {
                timestamp: Timestamp::from_unix_nanos(2_000),
                severity: Severity::Info,
                body: "handled".to_owned(),
                attributes: Attributes::new(),
                resource: Resource::service("api-gateway"),
                trace_id: Some(trace_id),
                span_id: Some(span_id),
            }
            .into(),
        );

        assert_eq!(engine.trace(&trace_id).len(), 1);
        assert_eq!(engine.logs_for_trace(&trace_id).len(), 1);
        assert_eq!(
            engine.logs_for_trace(&TraceId::from_bytes([0; 16])).len(),
            0
        );

        let stats = engine.stats();
        assert_eq!(stats.spans, 1);
        assert_eq!(stats.logs, 1);
    }
}
