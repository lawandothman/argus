//! Shared server state.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use argus_store::MemoryEngine;
use tokio::sync::RwLock;

/// State shared across request handlers: the storage engine (behind a read/write
/// lock) and the timestamp of the most recently ingested sample.
#[derive(Clone, Default)]
pub struct AppState {
    pub engine: Arc<RwLock<MemoryEngine>>,
    pub latest_ns: Arc<AtomicU64>,
}

impl AppState {
    pub fn new() -> Self {
        AppState::default()
    }
}
