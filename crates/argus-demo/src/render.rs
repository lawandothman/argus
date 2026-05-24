//! Renders a simulated request as a colored terminal block: a header line, a
//! span waterfall, the correlated logs, and a compact metrics line.

use argus_core::{MetricPoint, Severity, Span, SpanStatus};
use argus_ingest::demo::{RequestBatch, topology};

use crate::color;

const BAR_WIDTH: usize = 36;
const LABEL_WIDTH: usize = 16;

/// Render one request's telemetry into a multi-line, colored string.
pub fn render_request(batch: &RequestBatch) -> String {
    let root = batch.root();
    let total_ns = root.duration_nanos().max(1) as f64;
    let trace_start = root.start.as_unix_nanos();
    let mut out = String::new();

    let trace_hex = root.trace_id.to_hex();
    let trace_short = &trace_hex[..8];
    let status = if batch.failed {
        color::fg("● FAIL", (251, 113, 133))
    } else {
        color::fg("● OK", (52, 211, 153))
    };
    out.push_str(&format!(
        "{} {}   {}   {}\n",
        color::bold(&color::fg("▶ trace", (94, 234, 212))),
        color::dim(trace_short),
        color::bold(&format!("{:>7.1}ms", batch.duration_ms())),
        status,
    ));

    for span in &batch.spans {
        out.push_str(&render_span(span, trace_start, total_ns));
    }

    for log in &batch.logs {
        let (symbol, rgb) = severity_style(log.severity);
        out.push_str(&format!(
            "  {} {}\n",
            color::fg(symbol, rgb),
            color::dim(&log.body)
        ));
    }

    if !batch.metrics.is_empty() {
        let rendered: Vec<String> = batch.metrics.iter().map(render_metric).collect();
        out.push_str(&format!(
            "  {} {}\n",
            color::fg("▸", (120, 130, 150)),
            color::dim(&rendered.join("    ")),
        ));
    }

    out
}

/// Render a single span as a waterfall row: its bar is offset by start time and
/// sized by duration, both relative to the whole trace.
fn render_span(span: &Span, trace_start: u64, total_ns: f64) -> String {
    let name = span.resource.service_name().unwrap_or("?");
    let rgb = topology::color_for(name);

    let offset = ((span.start.as_unix_nanos().saturating_sub(trace_start) as f64) / total_ns
        * BAR_WIDTH as f64) as usize;
    let offset = offset.min(BAR_WIDTH.saturating_sub(1));
    let mut len = ((span.duration_nanos() as f64) / total_ns * BAR_WIDTH as f64).round() as usize;
    len = len.clamp(1, BAR_WIDTH - offset);

    let bar = format!("{}{}", " ".repeat(offset), "█".repeat(len));
    let bar = if span.status == SpanStatus::Error {
        color::fg(&bar, (251, 113, 133))
    } else {
        color::fg(&bar, rgb)
    };
    let duration = format!("{:>7.1}ms", span.duration_nanos() as f64 / 1_000_000.0);
    let pad = " ".repeat(LABEL_WIDTH.saturating_sub(name.len()));

    format!(
        "  {}{} {} {}\n",
        color::fg(name, rgb),
        pad,
        bar,
        color::dim(&duration)
    )
}

/// Render a metric as `name{labels} value`.
fn render_metric(metric: &MetricPoint) -> String {
    let labels: Vec<String> = metric
        .labels
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    let labels = if labels.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", labels.join(","))
    };
    format!("{}{} {:.0}", metric.name, labels, metric.sample.value)
}

/// A symbol and color for a log severity.
fn severity_style(severity: Severity) -> (&'static str, (u8, u8, u8)) {
    match severity {
        Severity::Trace => ("·", (120, 120, 120)),
        Severity::Debug => ("·", (130, 150, 170)),
        Severity::Info => ("ⓘ", (125, 170, 250)),
        Severity::Warn => ("▲", (251, 191, 36)),
        Severity::Error => ("✖", (251, 113, 133)),
        Severity::Fatal => ("✖", (255, 90, 90)),
    }
}
