//! A single metric time series: its identity plus its compressed samples.

use argus_core::{Labels, MetricKind, Sample, Timestamp};

use crate::gorilla::GorillaChunk;
use crate::query::TimeRange;

/// One time series, storing its samples Gorilla-compressed at millisecond
/// resolution.
#[derive(Debug)]
pub struct Series {
    pub name: String,
    pub labels: Labels,
    pub kind: MetricKind,
    chunk: GorillaChunk,
}

impl Series {
    pub fn new(name: String, labels: Labels, kind: MetricKind) -> Self {
        Series {
            name,
            labels,
            kind,
            chunk: GorillaChunk::new(),
        }
    }

    /// Append a sample (downsampled to millisecond resolution).
    pub fn append(&mut self, sample: Sample) {
        self.chunk
            .push(sample.timestamp.as_unix_millis(), sample.value);
    }

    /// Number of samples stored.
    pub fn len(&self) -> usize {
        self.chunk.count()
    }

    /// Compressed size of this series in bytes.
    pub fn encoded_len(&self) -> usize {
        self.chunk.encoded_len()
    }

    /// All samples whose timestamp falls within `range`.
    pub fn samples_in_range(&self, range: TimeRange) -> Vec<Sample> {
        self.chunk
            .samples()
            .into_iter()
            .map(|(millis, value)| Sample::new(Timestamp::from_unix_millis(millis), value))
            .filter(|sample| range.contains(sample.timestamp))
            .collect()
    }
}
