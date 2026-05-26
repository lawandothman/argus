//! Feeds the engine with synthetic telemetry: a backfill burst so the UI has
//! history on load, then a live trickle.

use std::sync::atomic::Ordering;
use std::time::Duration;

use argus_core::Timestamp;
use argus_ingest::demo::DemoGenerator;
use argus_store::Storage;

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
            state.engine.write().await.ingest_all(batch.into_signals());
            state.latest_ns.store(latest, Ordering::Relaxed);
            tokio::time::sleep(interval).await;
        }
    });
}
