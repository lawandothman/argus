//! A detected anomaly.

/// A single anomalous observation flagged by a [`Detector`](crate::Detector).
#[derive(Debug, Clone, PartialEq)]
pub struct Anomaly {
    /// The detector that flagged it.
    pub detector: &'static str,
    pub timestamp_ms: u64,
    pub observed: f64,
    /// The value the detector expected (its center / forecast / baseline).
    pub expected: f64,
    /// How far outside normal, in detector-specific units (sigmas for the
    /// control charts, multiples of the threshold for CUSUM).
    pub score: f64,
}
