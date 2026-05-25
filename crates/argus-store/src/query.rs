//! Query primitives for retrieving from the store.
//!
//! This is the storage-level retrieval surface — equality label matching and a
//! time range. The richer query language (regex matchers, aggregations, `rate`,
//! quantiles) lands in the query engine that sits on top of this.

use argus_core::{Labels, MetricKind, Sample, Timestamp};

/// A single label constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// Label equals value.
    Eq(String, String),
    /// Label does not equal value.
    Ne(String, String),
}

impl Matcher {
    pub fn eq(key: impl Into<String>, value: impl Into<String>) -> Self {
        Matcher::Eq(key.into(), value.into())
    }

    pub fn ne(key: impl Into<String>, value: impl Into<String>) -> Self {
        Matcher::Ne(key.into(), value.into())
    }
}

/// A metric name plus the label matchers that select its series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub metric: String,
    pub matchers: Vec<Matcher>,
}

impl Selector {
    pub fn new(metric: impl Into<String>) -> Self {
        Selector {
            metric: metric.into(),
            matchers: Vec::new(),
        }
    }

    /// Add a matcher, returning `self` for chaining.
    pub fn with(mut self, matcher: Matcher) -> Self {
        self.matchers.push(matcher);
        self
    }
}

/// An inclusive time window `[start, end]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl TimeRange {
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        TimeRange { start, end }
    }

    /// A range covering all of time.
    pub fn all() -> Self {
        TimeRange {
            start: Timestamp::EPOCH,
            end: Timestamp::from_unix_nanos(u64::MAX),
        }
    }

    pub fn contains(&self, timestamp: Timestamp) -> bool {
        self.start <= timestamp && timestamp <= self.end
    }
}

/// One series returned by a metric query: its identifying labels and the
/// samples that fell within the requested range.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesResult {
    pub metric: String,
    pub kind: MetricKind,
    pub labels: Labels,
    pub samples: Vec<Sample>,
}
