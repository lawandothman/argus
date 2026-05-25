//! Ingest synthetic telemetry into the in-memory engine and report what the
//! storage layer achieves: compression, a label-matched metric query with
//! quantiles, and a trace correlated to its logs.
//!
//! ```text
//! cargo run -p argus-store --example ingest           # 5,000 requests
//! cargo run -p argus-store --example ingest -- 20000
//! ```

use argus_core::TraceId;
use argus_ingest::demo::DemoGenerator;
use argus_store::{Matcher, MemoryEngine, Selector, Storage, TimeRange};

fn main() {
    let requests: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(5_000);

    let mut engine = MemoryEngine::new();
    let mut generator = DemoGenerator::new(7);

    let mut first_trace: Option<TraceId> = None;
    let mut failed_trace: Option<TraceId> = None;

    for _ in 0..requests {
        let batch = generator.next_request();
        first_trace.get_or_insert(batch.trace_id);
        if batch.failed {
            failed_trace.get_or_insert(batch.trace_id);
        }
        engine.ingest_all(batch.into_signals());
    }

    let stats = engine.stats();
    println!();
    println!("  ingested   {requests} requests");
    println!(
        "  stored     {} series · {} samples · {} spans / {} traces · {} logs",
        stats.series, stats.samples, stats.spans, stats.traces, stats.logs
    );
    println!(
        "  metrics    {} KB compressed  ({:.2} bytes/sample — {:.1}x smaller than raw i64+f64)",
        stats.metric_bytes / 1024,
        stats.bytes_per_sample(),
        stats.compression_ratio(),
    );

    // Label-matched retrieval + quantiles (the query language proper comes later).
    let mut latencies: Vec<f64> = engine
        .query_metrics(
            &Selector::new("http_request_duration_ms").with(Matcher::eq("status", "200")),
            TimeRange::all(),
        )
        .iter()
        .flat_map(|series| series.samples.iter().map(|sample| sample.value))
        .collect();
    let failures: usize = engine
        .query_metrics(
            &Selector::new("http_request_duration_ms").with(Matcher::eq("status", "500")),
            TimeRange::all(),
        )
        .iter()
        .map(|series| series.samples.len())
        .sum();

    let p50 = percentile(&mut latencies, 0.50);
    let p95 = percentile(&mut latencies, 0.95);
    let p99 = percentile(&mut latencies, 0.99);
    println!();
    println!("  query      http_request_duration_ms{{status=\"200\"}}");
    println!(
        "             p50 {p50:.0}ms · p95 {p95:.0}ms · p99 {p99:.0}ms   (n={})",
        latencies.len()
    );
    println!("             status=\"500\" failed requests: {failures}");

    // Cross-signal correlation, straight out of the store.
    if let Some(trace_id) = failed_trace.or(first_trace) {
        let spans = engine.trace(&trace_id);
        let logs = engine.logs_for_trace(&trace_id);
        println!();
        println!(
            "  trace      {}  ({} spans, {} correlated logs)",
            &trace_id.to_hex()[..8],
            spans.len(),
            logs.len(),
        );
        for span in &spans {
            println!(
                "               {:<12} {:<16} {:>7.1}ms  {:?}",
                span.resource.service_name().unwrap_or("?"),
                span.name,
                span.duration_nanos() as f64 / 1_000_000.0,
                span.status,
            );
        }
        for log in &logs {
            println!("               · [{:?}] {}", log.severity, log.body);
        }
    }
    println!();
}

/// The `p`-quantile (0.0–1.0) via nearest-rank on a sorted copy.
fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let rank = (p * (values.len() - 1) as f64).round() as usize;
    values[rank.min(values.len() - 1)]
}
