//! The unified telemetry envelope that flows through the ingest pipeline.

use serde::{Deserialize, Serialize};

use crate::{log::LogRecord, metric::MetricPoint, trace::Span};

/// Any single piece of telemetry: a metric point, a span, or a log record.
///
/// Ingest sources produce a stream of `Signal`s; the storage engine routes each
/// variant to the appropriate store. Keeping the three signals in one type lets
/// the pipeline treat them uniformly until the moment they are persisted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "signal", rename_all = "snake_case")]
pub enum Signal {
    Metric(MetricPoint),
    Span(Span),
    Log(LogRecord),
}

impl Signal {
    /// A short, stable label for the signal kind — handy for ingest-throughput
    /// metrics and structured logging.
    pub fn kind(&self) -> &'static str {
        match self {
            Signal::Metric(_) => "metric",
            Signal::Span(_) => "span",
            Signal::Log(_) => "log",
        }
    }
}

impl From<MetricPoint> for Signal {
    fn from(point: MetricPoint) -> Self {
        Signal::Metric(point)
    }
}

impl From<Span> for Signal {
    fn from(span: Span) -> Self {
        Signal::Span(span)
    }
}

impl From<LogRecord> for Signal {
    fn from(record: LogRecord) -> Self {
        Signal::Log(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        labels::Labels,
        metric::{MetricKind, MetricPoint, Sample},
        timestamp::Timestamp,
    };

    fn sample_point() -> MetricPoint {
        MetricPoint::new(
            "cpu_usage_ratio",
            MetricKind::Gauge,
            Labels::new().with("core", "0"),
            Sample::new(Timestamp::from_unix_nanos(42), 0.9),
        )
    }

    #[test]
    fn kind_label_matches_variant() {
        let signal: Signal = sample_point().into();
        assert_eq!(signal.kind(), "metric");
    }

    #[test]
    fn json_round_trip_preserves_variant() {
        let signal: Signal = sample_point().into();
        let json = serde_json::to_string(&signal).unwrap();
        let restored: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(signal, restored);
    }
}
