//! Detect anomalies in the demo system's latency and correlate the first
//! changepoint to the traces and logs that explain it.
//!
//! ```text
//! cargo run -p argus-anomaly --example detect
//! ```

use argus_anomaly::{Cusum, Detector, Ewma, Mad, explain};
use argus_core::Timestamp;
use argus_ingest::demo::DemoGenerator;
use argus_store::{Matcher, MemoryEngine, Selector, Storage, TimeRange};

fn main() {
    let requests: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(8_000);

    let mut engine = MemoryEngine::new();
    let mut generator = DemoGenerator::new(7);
    for _ in 0..requests {
        engine.ingest_all(generator.next_request().into_signals());
    }

    // The successful-request latency series — the demo periodically regresses
    // payments, which shows up here as a sustained step up.
    let series: Vec<(u64, f64)> = engine
        .query_metrics(
            &Selector::new("http_request_duration_ms").with(Matcher::eq("status", "200")),
            TimeRange::all(),
        )
        .iter()
        .flat_map(|result| {
            result
                .samples
                .iter()
                .map(|sample| (sample.timestamp.as_unix_millis(), sample.value))
        })
        .collect();

    println!(
        "\n  series  http_request_duration_ms{{status=\"200\"}}  ({} points)\n",
        series.len()
    );

    let ewma = Ewma::default();
    let mad = Mad::default();
    let cusum = Cusum::default();
    let ewma_hits = ewma.detect(&series);
    let mad_hits = mad.detect(&series);
    let cusum_hits = cusum.detect(&series);

    println!("  detectors");
    println!("    {:<22} {:>5} anomalies", ewma.name(), ewma_hits.len());
    println!("    {:<22} {:>5} anomalies", mad.name(), mad_hits.len());
    println!(
        "    {:<22} {:>5} changepoints",
        cusum.name(),
        cusum_hits.len()
    );

    let Some(change) = cusum_hits.first() else {
        println!("\n  no changepoint detected");
        return;
    };

    println!(
        "\n  ⚠ changepoint at t={}  —  observed {:.0}ms vs baseline {:.0}ms  ({:.1}× threshold)",
        change.timestamp_ms, change.observed, change.expected, change.score,
    );

    // Correlate the window just after the changepoint to its cause.
    let window = TimeRange::new(
        Timestamp::from_unix_millis(change.timestamp_ms.saturating_sub(200)),
        Timestamp::from_unix_millis(change.timestamp_ms + 1_500),
    );
    let explanation = explain(&engine, window, 5);

    println!("\n  correlated cause — slowest traces in the window");
    for trace in &explanation.slowest {
        println!(
            "    {}  {:<13} {:>7.1}ms   slowest: {} {} {:.1}ms{}",
            &trace.trace_id.to_hex()[..8],
            trace.root_op,
            trace.duration_ms,
            trace.slowest_service,
            trace.slowest_op,
            trace.slowest_ms,
            if trace.failed { "  [FAILED]" } else { "" },
        );
    }

    if !explanation.error_logs.is_empty() {
        println!("\n  error logs in the window");
        for log in explanation.error_logs.iter().take(4) {
            println!("    [{:?}] {}", log.severity, log.body);
        }
    }
    println!();
}
