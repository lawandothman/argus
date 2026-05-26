//! Shared server state.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use argus_store::MemoryEngine;
use tokio::sync::{RwLock, broadcast};

use crate::event::StreamEvent;

/// Capacity of the live-event broadcast channel.
const EVENT_BUFFER: usize = 256;

/// State shared across handlers: the storage engine (behind a read/write lock),
/// the timestamp of the latest ingested sample, and a broadcast channel of live
/// telemetry events for WebSocket subscribers.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<RwLock<MemoryEngine>>,
    pub latest_ns: Arc<AtomicU64>,
    pub events: broadcast::Sender<StreamEvent>,
}

impl AppState {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        AppState {
            engine: Arc::default(),
            latest_ns: Arc::default(),
            events,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::new()
    }
}
