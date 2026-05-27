//! OTLP → Argus mapping.
//!
//! Decodes OTLP trace export payloads (protobuf or OTLP/JSON) and maps
//! OpenTelemetry spans onto Argus's internal [`Span`] model. The mapping in
//! [`spans_from_export`] is transport-agnostic — the HTTP handler and (later)
//! the gRPC service both feed it the same decoded request.

use argus_core::{
    AttributeValue, Attributes, Resource, Span, SpanId, SpanKind, SpanStatus, Timestamp, TraceId,
};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::common::v1::{AnyValue, KeyValue, any_value};
use opentelemetry_proto::tonic::resource::v1::Resource as OtlpResource;
use opentelemetry_proto::tonic::trace::v1::Span as OtlpSpan;
use prost::Message;

/// Why an OTLP payload could not be decoded.
#[derive(Debug)]
pub enum OtlpError {
    Protobuf(prost::DecodeError),
    Json(serde_json::Error),
}

impl std::fmt::Display for OtlpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtlpError::Protobuf(error) => write!(f, "invalid OTLP protobuf: {error}"),
            OtlpError::Json(error) => write!(f, "invalid OTLP json: {error}"),
        }
    }
}

impl std::error::Error for OtlpError {}

/// Decode an OTLP/HTTP trace export body — OTLP/JSON when the content type says
/// so, protobuf otherwise (the OTLP/HTTP default).
pub fn decode_trace_export(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<ExportTraceServiceRequest, OtlpError> {
    if content_type.is_some_and(|ct| ct.starts_with("application/json")) {
        serde_json::from_slice(body).map_err(OtlpError::Json)
    } else {
        ExportTraceServiceRequest::decode(body).map_err(OtlpError::Protobuf)
    }
}

/// Flatten an OTLP trace export (resource → scope → span) into Argus spans,
/// attaching each span's resource. Spans with malformed trace/span ids are
/// dropped rather than failing the whole batch.
pub fn spans_from_export(request: ExportTraceServiceRequest) -> Vec<Span> {
    let mut spans = Vec::new();
    for resource_spans in request.resource_spans {
        let resource = map_resource(resource_spans.resource);
        for scope_spans in resource_spans.scope_spans {
            for span in scope_spans.spans {
                if let Some(mapped) = map_span(span, &resource) {
                    spans.push(mapped);
                }
            }
        }
    }
    spans
}

fn map_span(span: OtlpSpan, resource: &Resource) -> Option<Span> {
    let trace_id: [u8; 16] = span.trace_id.try_into().ok()?;
    let span_id: [u8; 8] = span.span_id.try_into().ok()?;
    // A root span carries an empty parent id, which simply fails the conversion.
    let parent_span_id = <[u8; 8]>::try_from(span.parent_span_id)
        .ok()
        .map(SpanId::from_bytes);

    Some(Span {
        trace_id: TraceId::from_bytes(trace_id),
        span_id: SpanId::from_bytes(span_id),
        parent_span_id,
        name: span.name,
        kind: map_kind(span.kind),
        start: Timestamp::from_unix_nanos(span.start_time_unix_nano),
        end: Timestamp::from_unix_nanos(span.end_time_unix_nano),
        status: map_status(span.status.map(|status| status.code)),
        attributes: map_attributes(span.attributes),
        resource: resource.clone(),
    })
}

fn map_kind(kind: i32) -> SpanKind {
    match kind {
        2 => SpanKind::Server,
        3 => SpanKind::Client,
        4 => SpanKind::Producer,
        5 => SpanKind::Consumer,
        _ => SpanKind::Internal,
    }
}

fn map_status(code: Option<i32>) -> SpanStatus {
    match code {
        Some(2) => SpanStatus::Error,
        Some(1) => SpanStatus::Ok,
        _ => SpanStatus::Unset,
    }
}

fn map_resource(resource: Option<OtlpResource>) -> Resource {
    let mut mapped = Resource::new();
    if let Some(resource) = resource {
        for kv in resource.attributes {
            if let Some(value) = kv.value.and_then(map_value) {
                mapped = mapped.with(kv.key, value);
            }
        }
    }
    mapped
}

fn map_attributes(attributes: Vec<KeyValue>) -> Attributes {
    let mut mapped = Attributes::new();
    for kv in attributes {
        if let Some(value) = kv.value.and_then(map_value) {
            mapped.insert(kv.key, value);
        }
    }
    mapped
}

fn map_value(value: AnyValue) -> Option<AttributeValue> {
    match value.value? {
        any_value::Value::StringValue(string) => Some(AttributeValue::String(string)),
        any_value::Value::BoolValue(boolean) => Some(AttributeValue::Bool(boolean)),
        any_value::Value::IntValue(int) => Some(AttributeValue::Int(int)),
        any_value::Value::DoubleValue(double) => Some(AttributeValue::Double(double)),
        any_value::Value::ArrayValue(array) => Some(AttributeValue::Array(
            array.values.into_iter().filter_map(map_value).collect(),
        )),
        // Nested kv-lists, raw bytes, and string-table indices have no scalar
        // Argus equivalent.
        any_value::Value::KvlistValue(_)
        | any_value::Value::BytesValue(_)
        | any_value::Value::StringValueStrindex(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_proto::tonic::common::v1::any_value::Value;
    use opentelemetry_proto::tonic::trace::v1::span::SpanKind as OtlpSpanKind;
    use opentelemetry_proto::tonic::trace::v1::status::StatusCode;
    use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, ScopeSpans, Status};

    fn string_kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_owned(),
            value: Some(AnyValue {
                value: Some(Value::StringValue(value.to_owned())),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn maps_resource_spans_onto_argus_spans() {
        let span = OtlpSpan {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            parent_span_id: vec![3u8; 8],
            name: "payments.charge".to_owned(),
            kind: OtlpSpanKind::Client as i32,
            start_time_unix_nano: 1_000,
            end_time_unix_nano: 5_000,
            attributes: vec![string_kv("payment.provider", "stripe")],
            status: Some(Status {
                code: StatusCode::Error as i32,
                message: "declined".to_owned(),
            }),
            ..Default::default()
        };
        let request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: Some(OtlpResource {
                    attributes: vec![string_kv("service.name", "checkout")],
                    ..Default::default()
                }),
                scope_spans: vec![ScopeSpans {
                    spans: vec![span],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let spans = spans_from_export(request);
        assert_eq!(spans.len(), 1);

        let span = &spans[0];
        assert_eq!(span.name, "payments.charge");
        assert_eq!(span.kind, SpanKind::Client);
        assert_eq!(span.status, SpanStatus::Error);
        assert_eq!(span.start.as_unix_nanos(), 1_000);
        assert!(span.parent_span_id.is_some());
        assert_eq!(span.resource.service_name(), Some("checkout"));
        assert_eq!(span.attributes.get_str("payment.provider"), Some("stripe"));
    }

    #[test]
    fn empty_parent_id_means_root_span() {
        let request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                scope_spans: vec![ScopeSpans {
                    spans: vec![OtlpSpan {
                        trace_id: vec![9u8; 16],
                        span_id: vec![8u8; 8],
                        name: "GET /checkout".to_owned(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let spans = spans_from_export(request);
        assert_eq!(spans.len(), 1);
        assert!(spans[0].parent_span_id.is_none());
        assert_eq!(spans[0].kind, SpanKind::Internal);
    }
}
