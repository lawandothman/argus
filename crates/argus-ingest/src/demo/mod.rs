//! A synthetic distributed system that emits correlated telemetry.
//!
//! The generator simulates a `checkout` request fanning out through a handful
//! of services, producing a trace (with nested spans), structured logs that
//! carry the trace context, and request metrics — all sharing one `trace_id`,
//! so the three signals can be correlated exactly as real telemetry would be.

pub mod generator;
pub mod topology;

pub use generator::{DemoGenerator, RequestBatch};
