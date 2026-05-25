//! Query result values.

use argus_core::Labels;

/// One element of an instant vector: a label set and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct InstantSample {
    pub labels: Labels,
    pub value: f64,
}

/// The result of evaluating a query: either a single scalar or an instant
/// vector (a value per matching series).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(f64),
    Vector(Vec<InstantSample>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Scalar(_) => "scalar",
            Value::Vector(_) => "vector",
        }
    }
}
