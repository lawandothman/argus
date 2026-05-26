//! EWMA control chart — flags points far from an exponentially-weighted mean,
//! using an exponentially-weighted variance to set the band. Good for gradual
//! drift away from normal.

use crate::anomaly::Anomaly;
use crate::detector::Detector;

#[derive(Debug, Clone, Copy)]
pub struct Ewma {
    /// Smoothing factor for the mean and variance (0–1; smaller = slower).
    pub alpha: f64,
    /// Band width in standard deviations.
    pub k: f64,
    /// Samples to learn from before flagging.
    pub warmup: usize,
}

impl Default for Ewma {
    fn default() -> Self {
        Ewma {
            alpha: 0.05,
            k: 3.5,
            warmup: 30,
        }
    }
}

impl Detector for Ewma {
    fn name(&self) -> &'static str {
        "EWMA control chart"
    }

    fn detect(&self, series: &[(u64, f64)]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        let mut mean = match series.first() {
            Some(&(_, value)) => value,
            None => return anomalies,
        };
        let mut variance: f64 = 0.0;

        for (index, &(timestamp, value)) in series.iter().enumerate().skip(1) {
            let deviation = value - mean;
            let sigma = variance.sqrt();
            if index >= self.warmup && sigma > 0.0 && deviation.abs() > self.k * sigma {
                anomalies.push(Anomaly {
                    detector: self.name(),
                    timestamp_ms: timestamp,
                    observed: value,
                    expected: mean,
                    score: deviation.abs() / sigma,
                });
            }
            // West's incremental EWMA mean and variance.
            mean += self.alpha * deviation;
            variance = (1.0 - self.alpha) * (variance + self.alpha * deviation * deviation);
        }
        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_drift_but_not_noise() {
        let mut series: Vec<(u64, f64)> = (0..200).map(|i| (i, 100.0 + (i % 3) as f64)).collect();
        // a sustained jump at the end
        for i in 200..220 {
            series.push((i, 180.0));
        }
        let hits = Ewma::default().detect(&series);
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|a| a.timestamp_ms >= 200));
    }
}
