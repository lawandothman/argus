//! The storage seam.
//!
//! Everything above the engine talks to this trait, so the in-memory backend
//! can later be swapped for an on-disk or distributed one without touching
//! ingest, query, or the API layer.

use argus_core::{LogRecord, Signal, Span, TraceId};

use crate::query::{Selector, SeriesResult, TimeRange};
use crate::stats::StorageStats;

pub trait Storage {
    /// Route a signal to the appropriate store.
    fn ingest(&mut self, signal: Signal);

    /// Retrieve the metric series matching `selector` within `range`.
    fn query_metrics(&self, selector: &Selector, range: TimeRange) -> Vec<SeriesResult>;

    /// The spans of a trace, ordered by start time.
    fn trace(&self, id: &TraceId) -> Vec<Span>;

    /// The logs correlated to a trace.
    fn logs_for_trace(&self, id: &TraceId) -> Vec<LogRecord>;

    /// Root spans (trace entry points) whose start falls within `range`.
    fn root_spans_in_range(&self, range: TimeRange) -> Vec<Span>;

    /// A snapshot of current contents and compression.
    fn stats(&self) -> StorageStats;

    /// Convenience: ingest a batch of signals.
    fn ingest_all(&mut self, signals: impl IntoIterator<Item = Signal>) {
        for signal in signals {
            self.ingest(signal);
        }
    }
}
