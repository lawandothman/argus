//! `argus-ingest` — telemetry ingest sources for Argus.
//!
//! Today this houses the synthetic [`demo`] generator, which simulates a small
//! distributed system emitting correlated metrics, traces, and logs. Real OTLP
//! and Prometheus receivers will join it later.

pub mod demo;
pub mod rng;
