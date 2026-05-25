//! An inverted index from metric names and labels to series ids.
//!
//! Selecting a series resolves to set operations: start with every series for
//! the metric name, intersect with each `Eq` matcher, and subtract each `Ne`.

use std::collections::{HashMap, HashSet};

use argus_core::{Labels, SeriesId};

use crate::query::{Matcher, Selector};

#[derive(Debug, Default)]
pub struct LabelIndex {
    by_name: HashMap<String, HashSet<SeriesId>>,
    by_label: HashMap<(String, String), HashSet<SeriesId>>,
}

impl LabelIndex {
    /// Register a series so it can be found by name and labels.
    pub fn insert(&mut self, id: SeriesId, name: &str, labels: &Labels) {
        self.by_name.entry(name.to_owned()).or_default().insert(id);
        for (key, value) in labels.iter() {
            self.by_label
                .entry((key.clone(), value.clone()))
                .or_default()
                .insert(id);
        }
    }

    /// Resolve a selector to the matching series ids.
    pub fn select(&self, selector: &Selector) -> HashSet<SeriesId> {
        let mut matched = match self.by_name.get(&selector.metric) {
            Some(set) => set.clone(),
            None => return HashSet::new(),
        };

        for matcher in &selector.matchers {
            match matcher {
                Matcher::Eq(key, value) => {
                    let key = (key.clone(), value.clone());
                    match self.by_label.get(&key) {
                        Some(set) => matched.retain(|id| set.contains(id)),
                        None => return HashSet::new(),
                    }
                }
                Matcher::Ne(key, value) => {
                    let key = (key.clone(), value.clone());
                    if let Some(set) = self.by_label.get(&key) {
                        matched.retain(|id| !set.contains(id));
                    }
                }
            }
        }

        matched
    }
}
