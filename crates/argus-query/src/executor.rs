//! The executor: walks a [`Plan`] against a [`Storage`] backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use argus_core::{Labels, Timestamp};
use argus_store::{Matcher as StoreMatcher, Selector, Storage, TimeRange};

use crate::ast::{AggOp, BinOp, Grouping};
use crate::error::QueryError;
use crate::function;
use crate::plan::Plan;
use crate::value::{InstantSample, Value};

pub fn evaluate<S: Storage>(plan: &Plan, store: &S, eval: Timestamp) -> Result<Value, QueryError> {
    match plan {
        Plan::Scalar(value) => Ok(Value::Scalar(*value)),

        Plan::Select {
            range_ms: Some(_), ..
        } => Err(QueryError::Eval(
            "range vector evaluated outside a function".into(),
        )),
        Plan::Select {
            metric,
            matchers,
            range_ms: None,
        } => {
            let vector = fetch(
                store,
                metric,
                matchers,
                TimeRange::new(Timestamp::EPOCH, eval),
            )
            .into_iter()
            .filter_map(|(labels, samples)| {
                samples
                    .last()
                    .map(|&(_, value)| InstantSample { labels, value })
            })
            .collect();
            Ok(Value::Vector(vector))
        }

        Plan::Func { name, param, input } => eval_func(name, *param, input, store, eval),

        Plan::Aggregate {
            op,
            grouping,
            param,
            input,
        } => {
            let vector = expect_vector(evaluate(input, store, eval)?)?;
            Ok(Value::Vector(aggregate(*op, grouping, *param, vector)))
        }

        Plan::Binary { op, lhs, rhs } => {
            let lhs = evaluate(lhs, store, eval)?;
            let rhs = evaluate(rhs, store, eval)?;
            eval_binary(*op, lhs, rhs)
        }

        Plan::Neg(inner) => Ok(match evaluate(inner, store, eval)? {
            Value::Scalar(value) => Value::Scalar(-value),
            Value::Vector(vector) => Value::Vector(map_values(vector, |value| -value)),
        }),
    }
}

fn eval_func<S: Storage>(
    name: &str,
    param: Option<f64>,
    input: &Plan,
    store: &S,
    eval: Timestamp,
) -> Result<Value, QueryError> {
    let Plan::Select {
        metric,
        matchers,
        range_ms: Some(range_ms),
    } = input
    else {
        return Err(QueryError::Eval(format!(
            "{name} expects a range vector argument"
        )));
    };

    let window_ns = range_ms.saturating_mul(1_000_000);
    let start = Timestamp::from_unix_nanos(eval.as_unix_nanos().saturating_sub(window_ns));
    let vector = fetch(store, metric, matchers, TimeRange::new(start, eval))
        .into_iter()
        .filter_map(|(labels, samples)| {
            function::apply_range(name, param, &samples)
                .map(|value| InstantSample { labels, value })
        })
        .collect();
    Ok(Value::Vector(vector))
}

/// Query the store and project each series to its `(timestamp_ms, value)` pairs.
fn fetch<S: Storage>(
    store: &S,
    metric: &str,
    matchers: &[StoreMatcher],
    range: TimeRange,
) -> Vec<(Labels, Vec<(u64, f64)>)> {
    let mut selector = Selector::new(metric);
    for matcher in matchers {
        selector = selector.with(matcher.clone());
    }
    store
        .query_metrics(&selector, range)
        .into_iter()
        .map(|series| {
            let samples = series
                .samples
                .iter()
                .map(|sample| (sample.timestamp.as_unix_millis(), sample.value))
                .collect();
            (series.labels, samples)
        })
        .collect()
}

fn aggregate(
    op: AggOp,
    grouping: &Grouping,
    param: Option<f64>,
    vector: Vec<InstantSample>,
) -> Vec<InstantSample> {
    let mut order: Vec<Labels> = Vec::new();
    let mut groups: HashMap<Labels, Vec<f64>> = HashMap::new();
    for sample in vector {
        let key = grouped_labels(&sample.labels, grouping);
        match groups.entry(key) {
            Entry::Occupied(mut slot) => slot.get_mut().push(sample.value),
            Entry::Vacant(slot) => {
                order.push(slot.key().clone());
                slot.insert(vec![sample.value]);
            }
        }
    }

    order
        .into_iter()
        .map(|labels| {
            let values = groups.remove(&labels).unwrap_or_default();
            InstantSample {
                value: reduce(op, param, values),
                labels,
            }
        })
        .collect()
}

fn reduce(op: AggOp, param: Option<f64>, mut values: Vec<f64>) -> f64 {
    match op {
        AggOp::Sum => values.iter().sum(),
        AggOp::Avg if values.is_empty() => f64::NAN,
        AggOp::Avg => values.iter().sum::<f64>() / values.len() as f64,
        AggOp::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        AggOp::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        AggOp::Count => values.len() as f64,
        AggOp::Quantile => param
            .and_then(|phi| function::quantile(phi, &mut values))
            .unwrap_or(f64::NAN),
    }
}

fn grouped_labels(labels: &Labels, grouping: &Grouping) -> Labels {
    let mut out = Labels::new();
    match grouping {
        Grouping::None => {}
        Grouping::By(keys) => {
            for (key, value) in labels.iter() {
                if keys.contains(key) {
                    out.insert(key.clone(), value.clone());
                }
            }
        }
        Grouping::Without(keys) => {
            for (key, value) in labels.iter() {
                if !keys.contains(key) {
                    out.insert(key.clone(), value.clone());
                }
            }
        }
    }
    out
}

fn eval_binary(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    match (lhs, rhs) {
        (Value::Scalar(a), Value::Scalar(b)) => Ok(Value::Scalar(op.apply(a, b))),
        (Value::Vector(vector), Value::Scalar(scalar)) => {
            Ok(Value::Vector(map_values(vector, |value| {
                op.apply(value, scalar)
            })))
        }
        (Value::Scalar(scalar), Value::Vector(vector)) => {
            Ok(Value::Vector(map_values(vector, |value| {
                op.apply(scalar, value)
            })))
        }
        (Value::Vector(_), Value::Vector(_)) => Err(QueryError::Unsupported(
            "binary operations between two vectors".into(),
        )),
    }
}

fn map_values(vector: Vec<InstantSample>, f: impl Fn(f64) -> f64) -> Vec<InstantSample> {
    vector
        .into_iter()
        .map(|sample| InstantSample {
            value: f(sample.value),
            labels: sample.labels,
        })
        .collect()
}

fn expect_vector(value: Value) -> Result<Vec<InstantSample>, QueryError> {
    match value {
        Value::Vector(vector) => Ok(vector),
        Value::Scalar(_) => Err(QueryError::Eval("expected a vector".into())),
    }
}

#[cfg(test)]
mod tests {
    use argus_core::{Labels, MetricKind, MetricPoint, Sample, Timestamp};
    use argus_store::{MemoryEngine, Storage};

    use crate::value::Value;

    fn counter(engine: &mut MemoryEngine, svc: &str, ts: u64, value: f64) {
        engine.ingest(
            MetricPoint::new(
                "reqs",
                MetricKind::Counter,
                Labels::new().with("svc", svc),
                Sample::new(Timestamp::from_unix_millis(ts), value),
            )
            .into(),
        );
    }

    #[test]
    fn evaluates_rate_of_a_counter() {
        let mut engine = MemoryEngine::new();
        for i in 0..6 {
            counter(&mut engine, "a", 1_000 + i * 1_000, i as f64);
        }
        let value = crate::run(
            "rate(reqs[1m])",
            &engine,
            Timestamp::from_unix_millis(6_000),
        )
        .unwrap();
        match value {
            Value::Vector(samples) => {
                assert_eq!(samples.len(), 1);
                assert!((samples[0].value - 1.0).abs() < 1e-9);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn evaluates_aggregation_with_grouping() {
        let mut engine = MemoryEngine::new();
        let ts = Timestamp::from_unix_millis(1_000);
        counter(&mut engine, "a", 1_000, 3.0);
        counter(&mut engine, "b", 1_000, 7.0);

        let value = crate::run("sum(reqs)", &engine, ts).unwrap();
        match value {
            Value::Vector(samples) => {
                assert_eq!(samples.len(), 1);
                assert!((samples[0].value - 10.0).abs() < 1e-9);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}
