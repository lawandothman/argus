//! `argus-ingest` — telemetry ingest sources for Argus.
//!
//! Today this houses the synthetic [`demo`] generator and an [`otlp`] receiver
//! that maps OpenTelemetry trace exports onto Argus's signal model. A Prometheus
//! receiver will join them later.

pub mod demo;
pub mod otlp;
pub mod rng;
