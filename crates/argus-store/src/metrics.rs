//! The metric store: a collection of compressed series plus the label index
//! used to find them.

use std::collections::HashMap;

use argus_core::{MetricPoint, SeriesId};

use crate::index::LabelIndex;
use crate::query::{Selector, SeriesResult, TimeRange};
use crate::series::Series;

#[derive(Debug, Default)]
pub struct MetricStore {
    series: HashMap<SeriesId, Series>,
    index: LabelIndex,
}

impl MetricStore {
    /// Append a point to its series, creating and indexing the series on first
    /// sight.
    pub fn append(&mut self, point: MetricPoint) {
        let id = point.series_id();
        if !self.series.contains_key(&id) {
            self.index.insert(id, &point.name, &point.labels);
            self.series.insert(
                id,
                Series::new(point.name.clone(), point.labels.clone(), point.kind),
            );
        }
        // Just inserted when absent, so the series is always present here.
        if let Some(series) = self.series.get_mut(&id) {
            series.append(point.sample);
        }
    }

    /// Retrieve the series matching `selector`, each with its samples in range.
    pub fn query(&self, selector: &Selector, range: TimeRange) -> Vec<SeriesResult> {
        self.index
            .select(selector)
            .into_iter()
            .filter_map(|id| self.series.get(&id))
            .map(|series| SeriesResult {
                metric: series.name.clone(),
                kind: series.kind,
                labels: series.labels.clone(),
                samples: series.samples_in_range(range),
            })
            .collect()
    }

    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    pub fn sample_count(&self) -> usize {
        self.series.values().map(Series::len).sum()
    }

    /// Total compressed size across all series, in bytes.
    pub fn encoded_len(&self) -> usize {
        self.series.values().map(Series::encoded_len).sum()
    }
}
