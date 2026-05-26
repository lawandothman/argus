//! The log store: log records plus a trace-id index for correlation.

use std::collections::HashMap;

use argus_core::{LogRecord, TraceId};

use crate::query::TimeRange;

#[derive(Debug, Default)]
pub struct LogStore {
    records: Vec<LogRecord>,
    by_trace: HashMap<TraceId, Vec<usize>>,
}

impl LogStore {
    pub fn insert(&mut self, record: LogRecord) {
        let position = self.records.len();
        if let Some(trace_id) = record.trace_id {
            self.by_trace.entry(trace_id).or_default().push(position);
        }
        self.records.push(record);
    }

    /// Every log emitted within the given trace — the cross-signal join.
    pub fn for_trace(&self, id: &TraceId) -> Vec<LogRecord> {
        self.by_trace
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|&position| self.records.get(position).cloned())
            .collect()
    }

    /// All logs whose timestamp falls within `range`.
    pub fn in_range(&self, range: TimeRange) -> Vec<LogRecord> {
        self.records
            .iter()
            .filter(|record| range.contains(record.timestamp))
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
}
