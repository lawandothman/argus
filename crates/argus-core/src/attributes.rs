//! The typed key/value attribute model shared by every signal.
//!
//! Mirrors the OpenTelemetry attribute model: arbitrary, typed key/value pairs
//! attached to spans, log records, and resources.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A single typed attribute value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
    Array(Vec<AttributeValue>),
}

impl AttributeValue {
    /// Borrow as a string slice, if this is an [`AttributeValue::String`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AttributeValue::String(value) => Some(value),
            _ => None,
        }
    }

    /// The integer, if this is an [`AttributeValue::Int`].
    pub fn as_int(&self) -> Option<i64> {
        match self {
            AttributeValue::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The float, if this is an [`AttributeValue::Double`].
    pub fn as_double(&self) -> Option<f64> {
        match self {
            AttributeValue::Double(value) => Some(*value),
            _ => None,
        }
    }

    /// The boolean, if this is an [`AttributeValue::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttributeValue::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        AttributeValue::String(value.to_owned())
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        AttributeValue::String(value)
    }
}

impl From<i64> for AttributeValue {
    fn from(value: i64) -> Self {
        AttributeValue::Int(value)
    }
}

impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        AttributeValue::Double(value)
    }
}

impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        AttributeValue::Bool(value)
    }
}

/// An ordered collection of typed attributes.
///
/// Backed by a [`BTreeMap`] so iteration order is deterministic — important for
/// stable hashing and reproducible serialization.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Attributes(BTreeMap<String, AttributeValue>);

impl Attributes {
    /// An empty attribute set.
    pub fn new() -> Self {
        Attributes(BTreeMap::new())
    }

    /// Insert or replace an attribute, returning `self` for chaining.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<AttributeValue>) -> Self {
        self.insert(key, value);
        self
    }

    /// Insert or replace an attribute.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<AttributeValue>) {
        self.0.insert(key.into(), value.into());
    }

    /// Look up an attribute by key.
    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.0.get(key)
    }

    /// Borrow an attribute as a string slice, if present and string-typed.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(AttributeValue::as_str)
    }

    /// Iterate over the attributes in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttributeValue)> {
        self.0.iter()
    }

    /// The number of attributes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no attributes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(String, AttributeValue)> for Attributes {
    fn from_iter<T: IntoIterator<Item = (String, AttributeValue)>>(iter: T) -> Self {
        Attributes(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_and_typed_accessors() {
        let attributes = Attributes::new()
            .with("http.method", "GET")
            .with("http.status_code", 200_i64)
            .with("error", false);

        assert_eq!(attributes.get_str("http.method"), Some("GET"));
        assert_eq!(
            attributes
                .get("http.status_code")
                .and_then(AttributeValue::as_int),
            Some(200)
        );
        assert_eq!(
            attributes.get("error").and_then(AttributeValue::as_bool),
            Some(false)
        );
        assert_eq!(attributes.len(), 3);
        assert!(!attributes.is_empty());
    }

    #[test]
    fn json_round_trip_preserves_values() {
        let attributes = Attributes::new()
            .with("service", "payments")
            .with("retries", 3_i64)
            .with("latency_ms", 12.5_f64)
            .with("ok", true);

        let json = serde_json::to_string(&attributes).unwrap();
        let restored: Attributes = serde_json::from_str(&json).unwrap();
        assert_eq!(attributes, restored);
    }
}
