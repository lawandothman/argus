//! The trace store: spans grouped by trace id.

use std::collections::HashMap;

use argus_core::{Span, TraceId};

#[derive(Debug, Default)]
pub struct TraceStore {
    by_trace: HashMap<TraceId, Vec<Span>>,
}

impl TraceStore {
    pub fn insert(&mut self, span: Span) {
        self.by_trace.entry(span.trace_id).or_default().push(span);
    }

    /// The spans of a trace, ordered by start time.
    pub fn trace(&self, id: &TraceId) -> Vec<Span> {
        let mut spans = self.by_trace.get(id).cloned().unwrap_or_default();
        spans.sort_by_key(|span| span.start);
        spans
    }

    pub fn trace_count(&self) -> usize {
        self.by_trace.len()
    }

    pub fn span_count(&self) -> usize {
        self.by_trace.values().map(Vec::len).sum()
    }
}
