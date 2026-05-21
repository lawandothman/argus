//! Label sets that identify a metric time series.
//!
//! Unlike [`Attributes`](crate::attributes::Attributes), labels are always
//! string-valued and canonically ordered, so a series has exactly one identity
//! — and one stable [`SeriesId`] — regardless of the order its labels were
//! supplied in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A canonically-ordered set of string labels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Labels(BTreeMap<String, String>);

impl Labels {
    /// An empty label set.
    pub fn new() -> Self {
        Labels(BTreeMap::new())
    }

    /// Insert or replace a label, returning `self` for chaining.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.insert(key, value);
        self
    }

    /// Insert or replace a label.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), value.into());
    }

    /// Look up a label value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Iterate over the labels in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter()
    }

    /// The number of labels.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no labels.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// A stable 64-bit identity for `metric_name` paired with this label set.
    ///
    /// Computed with FNV-1a over the canonical (key-ordered) encoding, so the
    /// same metric and labels always hash to the same [`SeriesId`] across
    /// processes and restarts — unlike the standard library's `Hash`, which is
    /// explicitly not guaranteed stable.
    pub fn series_id(&self, metric_name: &str) -> SeriesId {
        let mut hash = fnv::OFFSET_BASIS;
        fnv::write(&mut hash, metric_name.as_bytes());
        // A delimiter that cannot appear inside a UTF-8 string, so the metric
        // name and the first label can never run together ambiguously.
        fnv::write_byte(&mut hash, 0xff);
        for (key, value) in &self.0 {
            fnv::write(&mut hash, key.as_bytes());
            fnv::write_byte(&mut hash, b'=');
            fnv::write(&mut hash, value.as_bytes());
            fnv::write_byte(&mut hash, 0x00);
        }
        SeriesId(hash)
    }
}

impl FromIterator<(String, String)> for Labels {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Labels(iter.into_iter().collect())
    }
}

/// A stable, content-derived identifier for a metric time series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeriesId(u64);

impl SeriesId {
    /// The raw 64-bit value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// FNV-1a hashing — small, allocation-free, and stable across builds and
/// platforms (the standard library's hashers are neither guaranteed to be).
mod fnv {
    pub const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    #[inline]
    pub fn write_byte(hash: &mut u64, byte: u8) {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(PRIME);
    }

    #[inline]
    pub fn write(hash: &mut u64, bytes: &[u8]) {
        for &byte in bytes {
            write_byte(hash, byte);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_id_is_order_independent() {
        let a = Labels::new().with("method", "GET").with("route", "/pay");
        let b = Labels::new().with("route", "/pay").with("method", "GET");
        assert_eq!(a.series_id("http_requests"), b.series_id("http_requests"));
    }

    #[test]
    fn series_id_separates_name_from_labels() {
        let same_labels = Labels::new().with("route", "/pay");
        assert_ne!(
            same_labels.series_id("http_requests"),
            same_labels.series_id("http_errors")
        );
    }

    #[test]
    fn series_id_changes_with_label_values() {
        let pay = Labels::new().with("route", "/pay");
        let cart = Labels::new().with("route", "/cart");
        assert_ne!(
            pay.series_id("http_requests"),
            cart.series_id("http_requests")
        );
    }
}
