//! Anomaly → cause. Given the time window of an anomaly, surface the slowest
//! traces active in it and the error logs they emitted — turning "something is
//! wrong" into "here is what's wrong".

use argus_core::{LogRecord, Severity, Span, SpanStatus, TraceId};
use argus_store::{Storage, TimeRange};

/// A compact summary of one trace, with the slow span that dominated it.
#[derive(Debug, Clone)]
pub struct TraceSummary {
    pub trace_id: TraceId,
    pub root_op: String,
    pub duration_ms: f64,
    pub failed: bool,
    pub slowest_service: String,
    pub slowest_op: String,
    pub slowest_ms: f64,
}

/// The likely cause of an anomaly: its slowest traces and their error logs.
#[derive(Debug, Clone, Default)]
pub struct Explanation {
    pub slowest: Vec<TraceSummary>,
    pub error_logs: Vec<LogRecord>,
}

/// Explain an anomaly window by ranking the traces active in it by duration and
/// collecting the error logs they emitted.
pub fn explain<S: Storage>(store: &S, window: TimeRange, top_n: usize) -> Explanation {
    let mut roots = store.root_spans_in_range(window);
    roots.sort_by_key(|span| std::cmp::Reverse(span.duration_nanos()));
    roots.truncate(top_n);

    let mut explanation = Explanation::default();
    for root in roots {
        let spans = store.trace(&root.trace_id);
        // The span with the most *exclusive* work is the real bottleneck — not
        // whichever ancestor contains it (the root always wins on inclusive
        // duration, which tells you nothing).
        let slowest = slowest_by_self_time(&spans);
        let (slowest_service, slowest_op, slowest_ms) = match slowest {
            Some(span) => (
                span.resource.service_name().unwrap_or("?").to_owned(),
                span.name.clone(),
                span.duration_nanos() as f64 / 1_000_000.0,
            ),
            None => (String::new(), String::new(), 0.0),
        };

        explanation.slowest.push(TraceSummary {
            trace_id: root.trace_id,
            root_op: root.name.clone(),
            duration_ms: root.duration_nanos() as f64 / 1_000_000.0,
            failed: root.status == SpanStatus::Error,
            slowest_service,
            slowest_op,
            slowest_ms,
        });

        for log in store.logs_for_trace(&root.trace_id) {
            if log.severity >= Severity::Error {
                explanation.error_logs.push(log);
            }
        }
    }
    explanation
}

/// The span that did the most *exclusive* work (its own duration minus its
/// children's) — the actual bottleneck rather than an ancestor that contains it.
fn slowest_by_self_time(spans: &[Span]) -> Option<&Span> {
    spans.iter().max_by_key(|&span| self_time(span, spans))
}

fn self_time(span: &Span, spans: &[Span]) -> u64 {
    let children: u64 = spans
        .iter()
        .filter(|other| other.parent_span_id == Some(span.span_id))
        .map(Span::duration_nanos)
        .sum();
    span.duration_nanos().saturating_sub(children)
}
