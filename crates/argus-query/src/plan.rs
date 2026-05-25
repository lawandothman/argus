//! The planner: validates an AST and lowers it to an executable [`Plan`].
//!
//! Lowering resolves selectors, rejects unsupported constructs, and tracks each
//! node's value type so the executor never meets an ill-typed plan (e.g. a raw
//! range vector at the top level, or a binary op between two vectors).

use std::fmt;

use argus_store::Matcher as StoreMatcher;

use crate::ast::{AggOp, BinOp, Expr, Grouping, Matcher};
use crate::error::QueryError;

/// A validated, executable query plan.
#[derive(Debug, Clone)]
pub enum Plan {
    Scalar(f64),
    Select {
        metric: String,
        matchers: Vec<StoreMatcher>,
        range_ms: Option<u64>,
    },
    Func {
        name: String,
        param: Option<f64>,
        input: Box<Plan>,
    },
    Aggregate {
        op: AggOp,
        grouping: Grouping,
        param: Option<f64>,
        input: Box<Plan>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Plan>,
        rhs: Box<Plan>,
    },
    Neg(Box<Plan>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VType {
    Scalar,
    Vector,
    Range,
}

/// Lower an AST into a plan, rejecting a bare range-vector result.
pub fn lower(expr: &Expr) -> Result<Plan, QueryError> {
    let (plan, ty) = lower_typed(expr)?;
    if ty == VType::Range {
        return Err(QueryError::Unsupported(
            "result is a range vector; wrap it in a function such as rate()".into(),
        ));
    }
    Ok(plan)
}

fn lower_typed(expr: &Expr) -> Result<(Plan, VType), QueryError> {
    match expr {
        Expr::Number(value) => Ok((Plan::Scalar(*value), VType::Scalar)),
        Expr::Selector(selector) => lower_selector(selector),
        Expr::Call { func, args } => lower_call(func, args),
        Expr::Aggregate {
            op,
            grouping,
            param,
            arg,
        } => lower_aggregate(*op, grouping, param.as_deref(), arg),
        Expr::Binary { op, lhs, rhs } => lower_binary(*op, lhs, rhs),
        Expr::Neg(inner) => {
            let (plan, ty) = lower_typed(inner)?;
            if ty == VType::Range {
                return Err(QueryError::Unsupported(
                    "cannot negate a range vector".into(),
                ));
            }
            Ok((Plan::Neg(Box::new(plan)), ty))
        }
    }
}

fn lower_selector(selector: &crate::ast::Selector) -> Result<(Plan, VType), QueryError> {
    let mut matchers = Vec::with_capacity(selector.matchers.len());
    for matcher in &selector.matchers {
        matchers.push(match matcher {
            Matcher::Eq(key, value) => StoreMatcher::Eq(key.clone(), value.clone()),
            Matcher::Ne(key, value) => StoreMatcher::Ne(key.clone(), value.clone()),
            Matcher::ReEq(..) | Matcher::ReNe(..) => {
                return Err(QueryError::Unsupported(
                    "regex matchers (=~, !~) are not supported yet".into(),
                ));
            }
        });
    }
    let ty = if selector.range_ms.is_some() {
        VType::Range
    } else {
        VType::Vector
    };
    Ok((
        Plan::Select {
            metric: selector.metric.clone(),
            matchers,
            range_ms: selector.range_ms,
        },
        ty,
    ))
}

fn lower_call(func: &str, args: &[Expr]) -> Result<(Plan, VType), QueryError> {
    if !is_range_function(func) {
        return Err(QueryError::Unsupported(format!(
            "unknown function `{func}`"
        )));
    }

    let (param, input_expr) = if func == "quantile_over_time" {
        if args.len() != 2 {
            return Err(QueryError::Parse(
                "quantile_over_time expects (scalar, range vector)".into(),
            ));
        }
        let (param_plan, param_ty) = lower_typed(&args[0])?;
        if param_ty != VType::Scalar {
            return Err(QueryError::Parse(
                "quantile_over_time parameter must be a scalar".into(),
            ));
        }
        (
            Some(require_const(&param_plan, "quantile_over_time parameter")?),
            &args[1],
        )
    } else {
        if args.len() != 1 {
            return Err(QueryError::Parse(format!("{func} expects one argument")));
        }
        (None, &args[0])
    };

    let (input, input_ty) = lower_typed(input_expr)?;
    if input_ty != VType::Range {
        return Err(QueryError::Unsupported(format!(
            "{func} expects a range vector like metric[5m]"
        )));
    }
    Ok((
        Plan::Func {
            name: func.to_owned(),
            param,
            input: Box::new(input),
        },
        VType::Vector,
    ))
}

fn lower_aggregate(
    op: AggOp,
    grouping: &Grouping,
    param: Option<&Expr>,
    arg: &Expr,
) -> Result<(Plan, VType), QueryError> {
    let (input, input_ty) = lower_typed(arg)?;
    if input_ty != VType::Vector {
        return Err(QueryError::Unsupported(format!(
            "{} expects an instant vector",
            op.name()
        )));
    }

    let param = match (op, param) {
        (AggOp::Quantile, Some(expr)) => {
            let (plan, ty) = lower_typed(expr)?;
            if ty != VType::Scalar {
                return Err(QueryError::Parse(
                    "quantile parameter must be a scalar".into(),
                ));
            }
            Some(require_const(&plan, "quantile parameter")?)
        }
        (AggOp::Quantile, None) => {
            return Err(QueryError::Parse("quantile expects a parameter".into()));
        }
        _ => None,
    };

    Ok((
        Plan::Aggregate {
            op,
            grouping: grouping.clone(),
            param,
            input: Box::new(input),
        },
        VType::Vector,
    ))
}

fn lower_binary(op: BinOp, lhs: &Expr, rhs: &Expr) -> Result<(Plan, VType), QueryError> {
    let (lhs_plan, lhs_ty) = lower_typed(lhs)?;
    let (rhs_plan, rhs_ty) = lower_typed(rhs)?;
    if lhs_ty == VType::Range || rhs_ty == VType::Range {
        return Err(QueryError::Unsupported(
            "range vectors are only allowed inside functions".into(),
        ));
    }
    let ty = match (lhs_ty, rhs_ty) {
        (VType::Scalar, VType::Scalar) => VType::Scalar,
        (VType::Vector, VType::Scalar) | (VType::Scalar, VType::Vector) => VType::Vector,
        (VType::Vector, VType::Vector) => {
            return Err(QueryError::Unsupported(
                "binary operations between two vectors are not supported yet".into(),
            ));
        }
        _ => unreachable!("range types are rejected above"),
    };
    Ok((
        Plan::Binary {
            op,
            lhs: Box::new(lhs_plan),
            rhs: Box::new(rhs_plan),
        },
        ty,
    ))
}

fn require_const(plan: &Plan, context: &str) -> Result<f64, QueryError> {
    match plan {
        Plan::Scalar(value) => Ok(*value),
        _ => Err(QueryError::Unsupported(format!(
            "{context} must be a constant number"
        ))),
    }
}

fn is_range_function(name: &str) -> bool {
    matches!(
        name,
        "rate"
            | "increase"
            | "avg_over_time"
            | "sum_over_time"
            | "min_over_time"
            | "max_over_time"
            | "count_over_time"
            | "quantile_over_time"
    )
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_tree(f, 0)
    }
}

impl Plan {
    fn write_tree(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let pad = "  ".repeat(depth);
        match self {
            Plan::Scalar(value) => writeln!(f, "{pad}Scalar {value}"),
            Plan::Select {
                metric,
                matchers,
                range_ms,
            } => {
                writeln!(
                    f,
                    "{pad}Select {metric}{}{}",
                    format_matchers(matchers),
                    format_range(*range_ms)
                )
            }
            Plan::Func { name, param, input } => {
                writeln!(f, "{pad}Func {name}{}", format_param(*param))?;
                input.write_tree(f, depth + 1)
            }
            Plan::Aggregate {
                op,
                grouping,
                param,
                input,
            } => {
                writeln!(
                    f,
                    "{pad}Aggregate {}{}{}",
                    op.name(),
                    format_grouping(grouping),
                    format_param(*param)
                )?;
                input.write_tree(f, depth + 1)
            }
            Plan::Binary { op, lhs, rhs } => {
                writeln!(f, "{pad}Binary {}", op.symbol())?;
                lhs.write_tree(f, depth + 1)?;
                rhs.write_tree(f, depth + 1)
            }
            Plan::Neg(inner) => {
                writeln!(f, "{pad}Neg")?;
                inner.write_tree(f, depth + 1)
            }
        }
    }
}

fn format_matchers(matchers: &[StoreMatcher]) -> String {
    if matchers.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = matchers
        .iter()
        .map(|matcher| match matcher {
            StoreMatcher::Eq(key, value) => format!("{key}=\"{value}\""),
            StoreMatcher::Ne(key, value) => format!("{key}!=\"{value}\""),
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn format_range(range_ms: Option<u64>) -> String {
    match range_ms {
        None => String::new(),
        Some(ms) if ms % 3_600_000 == 0 => format!(" [{}h]", ms / 3_600_000),
        Some(ms) if ms % 60_000 == 0 => format!(" [{}m]", ms / 60_000),
        Some(ms) if ms % 1_000 == 0 => format!(" [{}s]", ms / 1_000),
        Some(ms) => format!(" [{ms}ms]"),
    }
}

fn format_param(param: Option<f64>) -> String {
    param
        .map(|value| format!(" (φ={value})"))
        .unwrap_or_default()
}

fn format_grouping(grouping: &Grouping) -> String {
    match grouping {
        Grouping::None => String::new(),
        Grouping::By(labels) => format!(" by ({})", labels.join(", ")),
        Grouping::Without(labels) => format!(" without ({})", labels.join(", ")),
    }
}
