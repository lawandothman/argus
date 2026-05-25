//! Range-vector functions: reduce a series' samples over a window to one value.

/// Apply a range function to a series' `(timestamp_ms, value)` samples.
/// Returns `None` when the window has too few samples to be meaningful.
pub fn apply_range(name: &str, param: Option<f64>, samples: &[(u64, f64)]) -> Option<f64> {
    match name {
        "rate" => rate(samples),
        "increase" => Some(increase(samples)),
        "avg_over_time" => mean(samples),
        "sum_over_time" => Some(samples.iter().map(|sample| sample.1).sum()),
        "min_over_time" => samples.iter().map(|sample| sample.1).reduce(f64::min),
        "max_over_time" => samples.iter().map(|sample| sample.1).reduce(f64::max),
        "count_over_time" => Some(samples.len() as f64),
        "quantile_over_time" => {
            let mut values: Vec<f64> = samples.iter().map(|sample| sample.1).collect();
            quantile(param?, &mut values)
        }
        _ => None,
    }
}

/// Total increase across the window, correcting for counter resets (a sample
/// lower than its predecessor is treated as a reset to zero).
fn increase(samples: &[(u64, f64)]) -> f64 {
    let mut iter = samples.iter();
    let Some(&(_, mut prev)) = iter.next() else {
        return 0.0;
    };
    let mut total = 0.0;
    for &(_, value) in iter {
        total += if value >= prev { value - prev } else { value };
        prev = value;
    }
    total
}

/// Per-second rate of increase across the window.
fn rate(samples: &[(u64, f64)]) -> Option<f64> {
    if samples.len() < 2 {
        return None;
    }
    let span_ms = samples.last()?.0 - samples.first()?.0;
    if span_ms == 0 {
        return None;
    }
    Some(increase(samples) / (span_ms as f64 / 1_000.0))
}

fn mean(samples: &[(u64, f64)]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    Some(samples.iter().map(|sample| sample.1).sum::<f64>() / samples.len() as f64)
}

/// The `phi`-quantile (0.0–1.0) with linear interpolation between ranks.
pub fn quantile(phi: f64, values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let phi = phi.clamp(0.0, 1.0);
    let rank = phi * (values.len() - 1) as f64;
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;
    if low == high {
        return Some(values[low]);
    }
    let frac = rank - low as f64;
    Some(values[low] * (1.0 - frac) + values[high] * frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_handles_a_clean_counter() {
        // 0,1,2,3,4,5 over 5s → 1.0/s
        let samples: Vec<(u64, f64)> = (0..6).map(|i| (1_000 + i * 1_000, i as f64)).collect();
        assert!((rate(&samples).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn increase_corrects_for_resets() {
        // 8 -> 10 (+2), reset to 1 (+1), -> 4 (+3) = 6
        let samples = [(0, 8.0), (1, 10.0), (2, 1.0), (3, 4.0)];
        assert!((increase(&samples) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn quantile_interpolates() {
        let mut values = [10.0, 20.0, 30.0, 40.0];
        assert!((quantile(0.5, &mut values).unwrap() - 25.0).abs() < 1e-9);
    }
}
