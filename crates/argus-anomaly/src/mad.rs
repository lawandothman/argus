//! Robust z-score via the median absolute deviation (MAD) over a trailing
//! window. Resistant to outliers, so a few spikes don't poison the baseline.

use crate::anomaly::Anomaly;
use crate::detector::Detector;
use crate::stats::median;

/// The consistency constant that makes MAD a standard-deviation estimate for
/// normally distributed data.
const MAD_TO_SIGMA: f64 = 0.6745;

#[derive(Debug, Clone, Copy)]
pub struct Mad {
    /// Trailing window size used to estimate the baseline.
    pub window: usize,
    /// Robust z-score threshold.
    pub threshold: f64,
}

impl Default for Mad {
    fn default() -> Self {
        Mad {
            window: 40,
            threshold: 4.0,
        }
    }
}

impl Detector for Mad {
    fn name(&self) -> &'static str {
        "robust z-score (MAD)"
    }

    fn detect(&self, series: &[(u64, f64)]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        if series.len() <= self.window {
            return anomalies;
        }

        for frame in series.windows(self.window + 1) {
            let (trailing, point) = frame.split_at(self.window);
            let mut baseline: Vec<f64> = trailing.iter().map(|sample| sample.1).collect();
            let center = median(&mut baseline);
            let mut deviations: Vec<f64> = baseline
                .iter()
                .map(|value| (value - center).abs())
                .collect();
            let mad = median(&mut deviations);
            if mad <= 0.0 {
                continue;
            }

            let (timestamp, value) = point[0];
            let z = MAD_TO_SIGMA * (value - center) / mad;
            if z.abs() > self.threshold {
                anomalies.push(Anomaly {
                    detector: self.name(),
                    timestamp_ms: timestamp,
                    observed: value,
                    expected: center,
                    score: z.abs(),
                });
            }
        }
        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_spike_against_a_steady_baseline() {
        let mut series: Vec<(u64, f64)> = (0..100).map(|i| (i, 50.0 + (i % 5) as f64)).collect();
        series.push((100, 500.0)); // obvious spike
        let hits = Mad::default().detect(&series);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].timestamp_ms, 100);
    }
}
