//! Additive Holt-Winters (triple exponential smoothing): models level, trend,
//! and a repeating seasonal pattern, then flags points whose residual from the
//! forecast is large. Good when "normal" itself rises, falls, and cycles.

use crate::anomaly::Anomaly;
use crate::detector::Detector;
use crate::stats::mean;

#[derive(Debug, Clone, Copy)]
pub struct HoltWinters {
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    /// Length of the seasonal cycle, in samples.
    pub period: usize,
    /// Band width in standard deviations of the forecast residual.
    pub k: f64,
}

impl HoltWinters {
    pub fn new(period: usize) -> Self {
        HoltWinters {
            alpha: 0.3,
            beta: 0.05,
            gamma: 0.3,
            period,
            k: 4.0,
        }
    }
}

impl Default for HoltWinters {
    fn default() -> Self {
        HoltWinters::new(12)
    }
}

impl Detector for HoltWinters {
    fn name(&self) -> &'static str {
        "Holt-Winters"
    }

    fn detect(&self, series: &[(u64, f64)]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        let period = self.period;
        if period == 0 || series.len() < 2 * period {
            return anomalies;
        }

        let values: Vec<f64> = series.iter().map(|sample| sample.1).collect();

        // Initialize from the first two seasons.
        let mut level = mean(&values[..period]);
        let season_two = mean(&values[period..2 * period]);
        let mut trend = (season_two - level) / period as f64;
        let mut seasonal: Vec<f64> = values[..period].iter().map(|value| value - level).collect();

        let mut residual_variance: f64 = 0.0;
        const RESIDUAL_ALPHA: f64 = 0.05;

        for (step, &(timestamp, value)) in series.iter().enumerate().skip(period) {
            let season_index = step % period;
            let forecast = level + trend + seasonal[season_index];
            let residual = value - forecast;
            let sigma = residual_variance.sqrt();

            if step >= 2 * period && sigma > 0.0 && residual.abs() > self.k * sigma {
                anomalies.push(Anomaly {
                    detector: self.name(),
                    timestamp_ms: timestamp,
                    observed: value,
                    expected: forecast,
                    score: residual.abs() / sigma,
                });
            }

            let previous_level = level;
            level = self.alpha * (value - seasonal[season_index])
                + (1.0 - self.alpha) * (level + trend);
            trend = self.beta * (level - previous_level) + (1.0 - self.beta) * trend;
            seasonal[season_index] =
                self.gamma * (value - level) + (1.0 - self.gamma) * seasonal[season_index];
            residual_variance =
                (1.0 - RESIDUAL_ALPHA) * residual_variance + RESIDUAL_ALPHA * residual * residual;
        }
        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn learns_seasonality_and_flags_the_break() {
        let period = 12;
        let mut series: Vec<(u64, f64)> = Vec::new();
        for i in 0..(period * 10) {
            let seasonal = 20.0 * (2.0 * PI * (i % period) as f64 / period as f64).sin();
            let noise = (i * 7 % 5) as f64 - 2.0; // small deterministic jitter
            series.push((i as u64, 100.0 + seasonal + noise));
        }
        // a clear break from the learned pattern
        let next = series.len() as u64;
        series.push((next, 250.0));

        let hits = HoltWinters::new(period).detect(&series);
        assert!(
            hits.iter().any(|a| a.timestamp_ms == next),
            "should flag the break"
        );
        // the regular seasonal points should not be flagged
        assert!(hits.iter().all(|a| a.observed > 200.0));
    }
}
