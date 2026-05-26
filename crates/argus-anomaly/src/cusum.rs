//! Tabular CUSUM — accumulates deviations from a baseline and fires when the
//! running sum crosses a threshold. The textbook detector for a *sustained*
//! shift in the mean, like the latency step from a bad deploy.

use crate::anomaly::Anomaly;
use crate::detector::Detector;
use crate::stats::{mean, stddev};

#[derive(Debug, Clone, Copy)]
pub struct Cusum {
    /// Samples used to estimate the baseline mean and standard deviation.
    pub warmup: usize,
    /// Slack, in standard deviations (shifts smaller than this are ignored).
    pub k_sigma: f64,
    /// Decision threshold, in standard deviations.
    pub h_sigma: f64,
}

impl Default for Cusum {
    fn default() -> Self {
        Cusum {
            warmup: 30,
            k_sigma: 0.5,
            h_sigma: 5.0,
        }
    }
}

impl Detector for Cusum {
    fn name(&self) -> &'static str {
        "CUSUM changepoint"
    }

    fn detect(&self, series: &[(u64, f64)]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        if series.len() <= self.warmup {
            return anomalies;
        }

        let baseline: Vec<f64> = series[..self.warmup]
            .iter()
            .map(|sample| sample.1)
            .collect();
        let target = mean(&baseline);
        let sigma = stddev(&baseline, target);
        if sigma <= 0.0 {
            return anomalies;
        }

        let slack = self.k_sigma * sigma;
        let threshold = self.h_sigma * sigma;
        let mut high = 0.0;
        let mut low = 0.0;

        for &(timestamp, value) in &series[self.warmup..] {
            high = (high + (value - target) - slack).max(0.0);
            low = (low + (target - value) - slack).max(0.0);
            let crossed = high.max(low);
            if crossed > threshold {
                anomalies.push(Anomaly {
                    detector: self.name(),
                    timestamp_ms: timestamp,
                    observed: value,
                    expected: target,
                    score: crossed / threshold,
                });
                // Reset so we can detect the next distinct changepoint.
                high = 0.0;
                low = 0.0;
            }
        }
        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_sustained_shift() {
        let mut series: Vec<(u64, f64)> = (0..100).map(|i| (i, 100.0 + (i % 3) as f64)).collect();
        for i in 100..160 {
            series.push((i, 200.0)); // sustained step up
        }
        let hits = Cusum::default().detect(&series);
        assert!(!hits.is_empty());
        // the first changepoint should land near the step at index 100
        assert!(hits[0].timestamp_ms >= 100 && hits[0].timestamp_ms < 110);
    }
}
