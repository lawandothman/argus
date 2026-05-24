//! Streams Argus's synthetic telemetry to the terminal — a runnable preview of
//! the data that flows through the platform.
//!
//! ```text
//! cargo run -p argus-demo            # live stream, ~900ms between requests
//! cargo run -p argus-demo -- 20      # emit 20 requests, then exit
//! ARGUS_INTERVAL_MS=200 cargo run -p argus-demo
//! ARGUS_SEED=42 cargo run -p argus-demo -- 5
//! ```

mod color;
mod render;

use std::io::Write;
use std::thread;
use std::time::Duration;

use argus_core::Timestamp;
use argus_ingest::demo::DemoGenerator;

fn main() {
    let seed = env_parse("ARGUS_SEED").unwrap_or_else(|| Timestamp::now().as_unix_nanos());
    let interval = Duration::from_millis(env_parse("ARGUS_INTERVAL_MS").unwrap_or(900));
    let limit: Option<u64> = std::env::args().nth(1).and_then(|arg| arg.parse().ok());

    let mut generator = DemoGenerator::new(seed);
    println!("{}", banner());

    let mut emitted = 0u64;
    loop {
        let batch = generator.next_request();
        print!("{}", render::render_request(&batch));
        println!();
        let _ = std::io::stdout().flush();

        emitted += 1;
        if limit.is_some_and(|limit| emitted >= limit) {
            break;
        }
        thread::sleep(interval);
    }
}

/// Parse an environment variable into a `u64`, if present and valid.
fn env_parse(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|value| value.parse().ok())
}

fn banner() -> String {
    let title = color::bold(&color::fg("◉ ARGUS", (94, 234, 212)));
    format!(
        "\n{title}  {}\n  {}\n",
        color::dim("· synthetic telemetry stream · Ctrl-C to stop"),
        color::dim("simulating  api-gateway → auth · catalog · cart → payments → postgres"),
    )
}
