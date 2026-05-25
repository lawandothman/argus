//! `argus-store` — the embedded storage engine for Argus.
//!
//! A single [`MemoryEngine`] holds all three signals: metrics as Gorilla-
//! compressed time series, spans indexed by trace id, and logs indexed by trace
//! id for correlation. Everything is reached through the [`Storage`] trait, so a
//! future on-disk or distributed backend is a swap, not a rewrite.
//!
//! On-disk segments + a write-ahead log are the next step; today the engine is
//! in-memory, but the compression and indexing are real.

mod bitstream;
mod engine;
mod gorilla;
mod index;
mod logs;
mod metrics;
mod query;
mod series;
mod stats;
mod storage;
mod traces;

pub use engine::MemoryEngine;
pub use query::{Matcher, Selector, SeriesResult, TimeRange};
pub use stats::StorageStats;
pub use storage::Storage;
