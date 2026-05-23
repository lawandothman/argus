//! Drives the simulated system, emitting one request's worth of correlated
//! telemetry at a time.

use argus_core::{
    Attributes, Labels, LogRecord, MetricKind, MetricPoint, Resource, Sample, Severity, Signal,
    Span, SpanId, SpanKind, SpanStatus, Timestamp, TraceId,
};

use crate::{
    demo::topology::{self, Call},
    rng::Rng,
};

/// All telemetry produced by one simulated request, grouped for correlation.
#[derive(Debug, Clone)]
pub struct RequestBatch {
    pub trace_id: TraceId,
    pub route: &'static str,
    /// Spans in the trace, root first.
    pub spans: Vec<Span>,
    pub metrics: Vec<MetricPoint>,
    pub logs: Vec<LogRecord>,
    pub failed: bool,
}

impl RequestBatch {
    /// The root (entry) span of the request.
    pub fn root(&self) -> &Span {
        &self.spans[0]
    }

    /// The end-to-end request duration in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        self.root().duration_nanos() as f64 / 1_000_000.0
    }

    /// Flatten into the signal stream that an ingest source would emit.
    pub fn into_signals(self) -> Vec<Signal> {
        let mut signals =
            Vec::with_capacity(self.spans.len() + self.metrics.len() + self.logs.len());
        signals.extend(self.spans.into_iter().map(Signal::from));
        signals.extend(self.metrics.into_iter().map(Signal::from));
        signals.extend(self.logs.into_iter().map(Signal::from));
        signals
    }
}

/// The synthetic telemetry generator.
#[derive(Debug, Clone)]
pub struct DemoGenerator {
    rng: Rng,
    clock: Timestamp,
    flow: Call,
    requests: u64,
}

impl DemoGenerator {
    /// Create a generator seeded for reproducibility.
    pub fn new(seed: u64) -> Self {
        DemoGenerator {
            rng: Rng::new(seed),
            clock: Timestamp::now(),
            flow: topology::checkout_flow(),
            requests: 0,
        }
    }

    /// How many requests have been emitted so far.
    pub fn requests_emitted(&self) -> u64 {
        self.requests
    }

    /// Produce the next request's telemetry, advancing the simulated clock by a
    /// realistic inter-arrival gap.
    pub fn next_request(&mut self) -> RequestBatch {
        let gap_ms = self.rng.range(20, 130) as f64;
        self.clock = advance(self.clock, gap_ms);
        self.requests += 1;

        let trace_id = TraceId::from_bytes(self.rng.bytes16());
        let start = self.clock;
        // Scenario: late in each cycle of 60 requests, a "deploy" regresses the
        // payments service — a visible p99 latency spike, just like real life.
        let payments_penalty_ms = if self.requests % 60 >= 45 { 95.0 } else { 0.0 };

        let flow = self.flow.clone();
        let mut building = Building {
            rng: &mut self.rng,
            trace_id,
            payments_penalty_ms,
            spans: Vec::new(),
            logs: Vec::new(),
        };
        let root_idx = building.build(&flow, None, start);
        let mut spans = building.spans;
        let mut logs = building.logs;

        let failed = spans.iter().any(|span| span.status == SpanStatus::Error);
        if failed {
            spans[root_idx].status = SpanStatus::Error;
        }

        let root_span_id = spans[root_idx].span_id;
        let duration_ms = spans[root_idx].duration_nanos() as f64 / 1_000_000.0;
        let status_code = if failed { "500" } else { "200" };
        let route = "/checkout";

        let metrics = vec![
            MetricPoint::new(
                "http_request_duration_ms",
                MetricKind::Gauge,
                Labels::new()
                    .with("route", route)
                    .with("status", status_code),
                Sample::new(start, duration_ms),
            ),
            MetricPoint::new(
                "http_requests_total",
                MetricKind::Counter,
                Labels::new()
                    .with("route", route)
                    .with("status", status_code),
                Sample::new(start, self.requests as f64),
            ),
        ];

        let verb = if failed { "failed" } else { "handled" };
        logs.insert(
            0,
            LogRecord {
                timestamp: start,
                severity: if failed {
                    Severity::Error
                } else {
                    Severity::Info
                },
                body: format!("{verb} GET {route} -> {status_code} in {duration_ms:.0}ms"),
                attributes: Attributes::new()
                    .with("http.route", route)
                    .with("http.status_code", status_code),
                resource: Resource::service("api-gateway"),
                trace_id: Some(trace_id),
                span_id: Some(root_span_id),
            },
        );

        RequestBatch {
            trace_id,
            route,
            spans,
            metrics,
            logs,
            failed,
        }
    }
}

/// Mutable state while assembling one request's trace.
struct Building<'a> {
    rng: &'a mut Rng,
    trace_id: TraceId,
    payments_penalty_ms: f64,
    spans: Vec<Span>,
    logs: Vec<LogRecord>,
}

impl Building<'_> {
    /// Recursively build the span for `call` (starting at `start`) and its
    /// children, returning the index of the created span in `self.spans`.
    fn build(&mut self, call: &Call, parent: Option<SpanId>, start: Timestamp) -> usize {
        let service = call.service;
        let span_id = SpanId::from_bytes(self.rng.bytes8());
        let index = self.spans.len();

        self.spans.push(Span {
            trace_id: self.trace_id,
            span_id,
            parent_span_id: parent,
            name: service.op.to_owned(),
            kind: if call.children.is_empty() {
                SpanKind::Client
            } else {
                SpanKind::Server
            },
            start,
            end: start,
            status: SpanStatus::Ok,
            attributes: Attributes::new().with("service.name", service.name),
            resource: Resource::service(service.name),
        });

        // Children run sequentially, each after a small dispatch delay.
        let mut cursor = advance(start, 1.0 + self.rng.next_f64());
        for child in &call.children {
            let child_index = self.build(child, Some(span_id), cursor);
            cursor = advance(self.spans[child_index].end, 0.5);
        }

        // This service's own work, after its downstream calls return.
        let mut self_ms =
            service.base_latency_ms + (self.rng.next_f64() - 0.5) * 2.0 * service.jitter_ms;
        if service.name == "payments" {
            self_ms += self.payments_penalty_ms;
        }
        let end = advance(cursor, self_ms.max(0.5));

        let errored = self.rng.chance(service.error_rate);
        if errored {
            self.logs.push(LogRecord {
                timestamp: end,
                severity: Severity::Error,
                body: format!("{}: {}", service.name, error_message(service.name)),
                attributes: Attributes::new().with("service.name", service.name),
                resource: Resource::service(service.name),
                trace_id: Some(self.trace_id),
                span_id: Some(span_id),
            });
        }

        let span = &mut self.spans[index];
        span.end = end;
        span.status = if errored {
            SpanStatus::Error
        } else {
            SpanStatus::Ok
        };
        index
    }
}

/// Advance a timestamp by a number of (fractional) milliseconds.
fn advance(at: Timestamp, millis: f64) -> Timestamp {
    Timestamp::from_unix_nanos(at.as_unix_nanos() + (millis * 1_000_000.0) as u64)
}

/// A plausible error message for a given service.
fn error_message(service: &str) -> &'static str {
    match service {
        "payments" => "card authorization declined by processor",
        "postgres" => "connection pool exhausted",
        "catalog" => "upstream inventory service timed out",
        "auth" => "token validation failed",
        "cart" => "cart snapshot is stale",
        _ => "internal error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_produces_the_full_trace() {
        let mut generator = DemoGenerator::new(1);
        let batch = generator.next_request();
        // gateway, auth, catalog, cart, payments, postgres.
        assert_eq!(batch.spans.len(), 6);
        assert!(batch.root().is_root());
        // Every span shares the trace id; children come after the root starts.
        for span in &batch.spans {
            assert_eq!(span.trace_id, batch.trace_id);
            assert!(span.start.as_unix_nanos() >= batch.root().start.as_unix_nanos());
        }
    }

    #[test]
    fn logs_carry_the_trace_context() {
        let mut generator = DemoGenerator::new(2);
        let batch = generator.next_request();
        assert!(
            batch
                .logs
                .iter()
                .all(|log| log.trace_id == Some(batch.trace_id))
        );
    }

    #[test]
    fn signals_cover_every_emitted_item() {
        let mut generator = DemoGenerator::new(3);
        let batch = generator.next_request();
        let expected = batch.spans.len() + batch.metrics.len() + batch.logs.len();
        assert_eq!(batch.into_signals().len(), expected);
    }
}
