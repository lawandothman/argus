//! Distributed tracing primitives: trace and span identifiers, and spans.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{attributes::Attributes, resource::Resource, timestamp::Timestamp};

/// A 16-byte trace identifier (W3C Trace Context / OTLP).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId([u8; 16]);

/// An 8-byte span identifier (W3C Trace Context / OTLP).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId([u8; 8]);

/// Implements the shared surface of a fixed-width, hex-encoded identifier: byte
/// conversions, hex parsing/formatting, and a wire representation that is a
/// lowercase hex string (faithful to the OTLP JSON encoding).
macro_rules! hex_id {
    ($name:ident, $len:literal, $err:literal) => {
        impl $name {
            /// The all-zero identifier — the OTLP "unset" sentinel.
            pub const ZERO: $name = $name([0; $len]);

            /// Construct from raw bytes.
            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                $name(bytes)
            }

            /// The raw bytes.
            pub const fn to_bytes(self) -> [u8; $len] {
                self.0
            }

            /// Whether every byte is zero.
            pub fn is_zero(&self) -> bool {
                self.0 == [0; $len]
            }

            /// Lowercase hex encoding.
            pub fn to_hex(&self) -> String {
                hex::encode(&self.0)
            }

            /// Parse from a hex string, returning `None` on the wrong length or
            /// any non-hex character.
            pub fn from_hex(s: &str) -> Option<Self> {
                hex::decode::<$len>(s).map($name)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.to_hex())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.to_hex())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_hex())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let encoded = String::deserialize(deserializer)?;
                $name::from_hex(&encoded).ok_or_else(|| de::Error::custom($err))
            }
        }
    };
}

hex_id!(
    TraceId,
    16,
    "invalid trace id: expected 32 lowercase hex characters"
);
hex_id!(
    SpanId,
    8,
    "invalid span id: expected 16 lowercase hex characters"
);

/// Lowercase hex encoding/decoding for fixed-width identifiers.
mod hex {
    /// Encode bytes as a lowercase hex string.
    pub fn encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            out.push(nibble(byte >> 4));
            out.push(nibble(byte & 0x0f));
        }
        out
    }

    /// Decode a hex string into exactly `N` bytes, or `None` on a length
    /// mismatch or any non-hex character.
    pub fn decode<const N: usize>(s: &str) -> Option<[u8; N]> {
        if s.len() != N * 2 {
            return None;
        }
        let mut out = [0u8; N];
        for (slot, pair) in out.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
            *slot = (from_digit(pair[0])? << 4) | from_digit(pair[1])?;
        }
        Some(out)
    }

    fn nibble(value: u8) -> char {
        match value {
            0..=9 => (b'0' + value) as char,
            _ => (b'a' + value - 10) as char,
        }
    }

    fn from_digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
}

/// The role a span plays in a trace (OpenTelemetry `SpanKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    #[default]
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// The outcome of the operation a span represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    #[default]
    Unset,
    Ok,
    Error,
}

/// A single operation within a trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    pub start: Timestamp,
    pub end: Timestamp,
    pub status: SpanStatus,
    pub attributes: Attributes,
    pub resource: Resource,
}

impl Span {
    /// Whether this span is the root of its trace (it has no parent).
    pub fn is_root(&self) -> bool {
        self.parent_span_id.is_none()
    }

    /// The span's duration in nanoseconds, saturating at zero when `end`
    /// precedes `start` (clock skew between hosts).
    pub fn duration_nanos(&self) -> u64 {
        self.end.saturating_nanos_since(self.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_hex_round_trip() {
        let id = TraceId::from_bytes([
            0x4b, 0xf9, 0x2f, 0x35, 0x77, 0xb3, 0x4d, 0xa6, 0xa3, 0xce, 0x92, 0x9d, 0x0e, 0x0e,
            0x47, 0x36,
        ]);
        assert_eq!(id.to_hex(), "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(TraceId::from_hex(&id.to_hex()), Some(id));
    }

    #[test]
    fn from_hex_rejects_bad_input() {
        assert_eq!(TraceId::from_hex("abc"), None); // wrong length
        assert_eq!(TraceId::from_hex(&"zz".repeat(16)), None); // non-hex characters
        assert_eq!(SpanId::from_hex("00"), None); // wrong length for a span id
    }

    #[test]
    fn ids_serialize_as_hex_strings() {
        let id = TraceId::from_bytes([0xab; 16]);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"abababababababababababababababab\"");
        assert_eq!(serde_json::from_str::<TraceId>(&json).unwrap(), id);
    }

    #[test]
    fn span_duration_and_root() {
        let span = Span {
            trace_id: TraceId::from_bytes([1; 16]),
            span_id: SpanId::from_bytes([2; 8]),
            parent_span_id: None,
            name: "GET /pay".to_owned(),
            kind: SpanKind::Server,
            start: Timestamp::from_unix_nanos(1_000),
            end: Timestamp::from_unix_nanos(4_000),
            status: SpanStatus::Ok,
            attributes: Attributes::new(),
            resource: Resource::service("payments"),
        };
        assert!(span.is_root());
        assert_eq!(span.duration_nanos(), 3_000);
    }
}
