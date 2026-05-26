//! `argus-anomaly` — explainable anomaly detection for Argus.
//!
//! Four detectors, each a transparent statistical method (no black-box ML), all
//! implementing the [`Detector`] trait over a `(timestamp_ms, value)` series:
//!
//! - [`Ewma`] — exponentially-weighted control chart, for gradual drift.
//! - [`Mad`] — robust z-score via median absolute deviation, for spikes.
//! - [`HoltWinters`] — triple exponential smoothing, for trend + seasonality.
//! - [`Cusum`] — cumulative sum, for sustained shifts like a bad deploy.
//!
//! When a detector fires, [`explain`] joins the anomaly window back to the
//! traces and logs that explain it — anomaly to cause, automatically.

mod anomaly;
mod correlate;
mod cusum;
mod detector;
mod ewma;
mod holt_winters;
mod mad;
mod stats;

pub use anomaly::Anomaly;
pub use correlate::{Explanation, TraceSummary, explain};
pub use cusum::Cusum;
pub use detector::Detector;
pub use ewma::Ewma;
pub use holt_winters::HoltWinters;
pub use mad::Mad;
