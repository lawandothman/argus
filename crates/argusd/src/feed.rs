//! Feeds the engine with synthetic telemetry: a backfill burst so the UI has
//! history on load, then a live trickle that also broadcasts to subscribers.

use std::sync::atomic::Ordering;
use std::time::Duration;

use argus_core::Timestamp;
use argus_ingest::demo::DemoGenerator;
use argus_store::Storage;

use crate::event::StreamEvent;
use crate::state::AppState;

/// Backfill `backfill` requests, then keep emitting one every `interval`.
pub async fn start(state: AppState, backfill: usize, interval: Duration) {
    let mut generator = DemoGenerator::new(Timestamp::now().as_unix_nanos());
    seed(&state, &mut generator, backfill).await;
    run_live(state, generator, interval);
}

async fn seed(state: &AppState, generator: &mut DemoGenerator, count: usize) {
    let mut engine = state.engine.write().await;
    let mut latest = 0;
    for _ in 0..count {
        let batch = generator.next_request();
        latest = batch.root().start.as_unix_nanos();
        engine.ingest_all(batch.into_signals());
    }
    state.latest_ns.store(latest, Ordering::Relaxed);
}

fn run_live(state: AppState, mut generator: DemoGenerator, interval: Duration) {
    tokio::spawn(async move {
        loop {
            let batch = generator.next_request();
            let latest = batch.root().start.as_unix_nanos();
            let event = StreamEvent::Request {
                trace_id: batch.trace_id.to_hex(),
                route: batch.route.to_owned(),
                status: if batch.failed { 500 } else { 200 },
                duration_ms: batch.duration_ms(),
                failed: batch.failed,
                timestamp_ns: latest,
            };
            state.engine.write().await.ingest_all(batch.into_signals());
            state.latest_ns.store(latest, Ordering::Relaxed);
            let _ = state.events.send(event);
            tokio::time::sleep(interval).await;
        }
    });
}
