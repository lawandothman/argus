//! The detector interface.

use crate::anomaly::Anomaly;

/// A statistical anomaly detector over a `(timestamp_ms, value)` series.
///
/// Implementations assume the series is sorted by timestamp ascending.
pub trait Detector {
    /// A short, stable name (used to label the anomalies it produces).
    fn name(&self) -> &'static str;

    /// Scan the series and return every anomalous point found.
    fn detect(&self, series: &[(u64, f64)]) -> Vec<Anomaly>;
}
