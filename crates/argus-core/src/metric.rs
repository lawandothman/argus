//! Metric samples and the points that carry them.

use serde::{Deserialize, Serialize};

use crate::{
    labels::{Labels, SeriesId},
    timestamp::Timestamp,
};

/// The semantics of a metric, following the OpenTelemetry data model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// A value that can rise and fall (e.g. temperature, queue depth).
    Gauge,
    /// A monotonically increasing cumulative total (e.g. requests served).
    Counter,
    /// A distribution, ingested as its component bucket/sum/count series.
    Histogram,
}

/// A single observation: a value at a point in time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub timestamp: Timestamp,
    pub value: f64,
}

impl Sample {
    pub fn new(timestamp: Timestamp, value: f64) -> Self {
        Sample { timestamp, value }
    }
}

/// A metric data point as it arrives on the wire: the metric name, the labels
/// that identify its series, and a single sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub kind: MetricKind,
    pub labels: Labels,
    pub sample: Sample,
}

impl MetricPoint {
    pub fn new(name: impl Into<String>, kind: MetricKind, labels: Labels, sample: Sample) -> Self {
        MetricPoint {
            name: name.into(),
            kind,
            labels,
            sample,
        }
    }

    /// The stable identity of the series this point belongs to.
    pub fn series_id(&self) -> SeriesId {
        self.labels.series_id(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_series_id_matches_its_labels() {
        let labels = Labels::new().with("route", "/pay");
        let point = MetricPoint::new(
            "http_requests_total",
            MetricKind::Counter,
            labels.clone(),
            Sample::new(Timestamp::from_unix_nanos(1), 1.0),
        );
        assert_eq!(point.series_id(), labels.series_id("http_requests_total"));
    }

    #[test]
    fn json_round_trip() {
        let point = MetricPoint::new(
            "cpu_usage_ratio",
            MetricKind::Gauge,
            Labels::new().with("core", "0"),
            Sample::new(Timestamp::from_unix_millis(1_700_000_000_000), 0.42),
        );
        let json = serde_json::to_string(&point).unwrap();
        let restored: MetricPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(point, restored);
    }
}
