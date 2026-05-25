//! A snapshot of what the engine is holding and how well it compresses.

/// Counters describing the engine's current contents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageStats {
    pub series: usize,
    pub samples: usize,
    pub spans: usize,
    pub traces: usize,
    pub logs: usize,
    /// Compressed size of all metric samples, in bytes.
    pub metric_bytes: usize,
}

impl StorageStats {
    /// What the samples would cost stored naively as `(i64, f64)` pairs.
    pub fn raw_metric_bytes(&self) -> usize {
        self.samples * (size_of::<i64>() + size_of::<f64>())
    }

    /// Raw size divided by compressed size.
    pub fn compression_ratio(&self) -> f64 {
        if self.metric_bytes == 0 {
            return 0.0;
        }
        self.raw_metric_bytes() as f64 / self.metric_bytes as f64
    }

    /// Average compressed bytes per sample.
    pub fn bytes_per_sample(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }
        self.metric_bytes as f64 / self.samples as f64
    }
}
