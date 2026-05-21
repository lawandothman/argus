//! `argus-core` — the shared telemetry data model for the Argus observability
//! platform.
//!
//! Every other crate (ingest, storage, query, anomaly, server, clients) speaks
//! in terms of the types defined here. The three signals — [`MetricPoint`],
//! [`Span`], and [`LogRecord`] — each carry a [`Resource`] and typed
//! [`Attributes`], and logs and spans share [`TraceId`] / [`SpanId`] so the
//! signals can be correlated. [`Signal`] is the envelope that unifies them on
//! the ingest path.

pub mod attributes;
pub mod labels;
pub mod log;
pub mod metric;
pub mod resource;
pub mod signal;
pub mod timestamp;
pub mod trace;

pub use attributes::{AttributeValue, Attributes};
pub use labels::{Labels, SeriesId};
pub use log::{LogRecord, Severity};
pub use metric::{MetricKind, MetricPoint, Sample};
pub use resource::Resource;
pub use signal::Signal;
pub use timestamp::Timestamp;
pub use trace::{Span, SpanId, SpanKind, SpanStatus, TraceId};
