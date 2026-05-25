//! Run a handful of PromQL-lite queries against demo telemetry, printing each
//! result — and the execution plan for one — to show the engine end to end.
//!
//! ```text
//! cargo run -p argus-query --example query
//! ```

use argus_core::{Labels, Timestamp};
use argus_ingest::demo::DemoGenerator;
use argus_query::{Value, plan, run};
use argus_store::{MemoryEngine, Storage};

fn main() {
    let requests: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(8_000);

    let mut engine = MemoryEngine::new();
    let mut generator = DemoGenerator::new(7);
    let mut eval = Timestamp::EPOCH;
    for _ in 0..requests {
        let batch = generator.next_request();
        eval = batch.root().start;
        engine.ingest_all(batch.into_signals());
    }
    println!("\n  ingested {requests} requests; evaluating at the latest timestamp\n");

    let queries = [
        r#"http_request_duration_ms{status="200"}"#,
        r#"avg_over_time(http_request_duration_ms{status="200"}[10m])"#,
        r#"quantile_over_time(0.95, http_request_duration_ms{status="200"}[10m])"#,
        r#"avg_over_time(http_request_duration_ms{status="200"}[10m]) / 1000"#,
        r#"rate(http_requests_total[5m])"#,
        r#"sum by (status) (count_over_time(http_request_duration_ms[10m]))"#,
    ];

    for query in queries {
        println!("  query   {query}");
        match run(query, &engine, eval) {
            Ok(value) => print_value(&value),
            Err(error) => println!("          error: {error}"),
        }
        println!();
    }

    let explained = r#"sum by (status) (count_over_time(http_request_duration_ms[10m]))"#;
    println!("  ── explain ────────────────────────────────────────");
    println!("  query   {explained}\n");
    match plan(explained) {
        Ok(plan) => print!("{}", indent(&plan.to_string())),
        Err(error) => println!("  error: {error}"),
    }
    println!();
}

fn print_value(value: &Value) {
    match value {
        Value::Scalar(scalar) => println!("          = {scalar:.4}"),
        Value::Vector(samples) if samples.is_empty() => println!("          (no matching series)"),
        Value::Vector(samples) => {
            for sample in samples {
                println!(
                    "          {:<34} {:>12.3}",
                    format_labels(&sample.labels),
                    sample.value
                );
            }
        }
    }
}

fn format_labels(labels: &Labels) -> String {
    let parts: Vec<String> = labels
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("          {line}\n"))
        .collect()
}
