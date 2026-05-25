//! `argus-query` — a PromQL-lite query engine for Argus.
//!
//! A query string travels through four stages, each in its own module:
//!
//! ```text
//! "sum by (status) (rate(http_requests_total[5m]))"
//!     lexer  →  parser  →  planner  →  executor
//!     tokens     AST        Plan        Value
//! ```
//!
//! The planner validates and lowers the AST (resolving selectors, checking
//! function arity and value types) so the executor only ever walks a plan it
//! can run. Supported today: instant/range selectors with `=`/`!=` matchers,
//! the `*_over_time` family plus `rate`/`increase`, aggregations
//! (`sum`/`avg`/`min`/`max`/`count`/`quantile`) with `by`/`without`, and
//! arithmetic between a vector and a scalar.

mod ast;
mod error;
mod executor;
mod function;
mod lexer;
mod parser;
mod plan;
mod token;
mod value;

use argus_core::Timestamp;
use argus_store::Storage;

pub use error::QueryError;
pub use plan::Plan;
pub use value::{InstantSample, Value};

/// Lex and parse a query string into an AST.
pub fn parse(input: &str) -> Result<ast::Expr, QueryError> {
    let tokens = lexer::lex(input)?;
    parser::parse(tokens)
}

/// Validate and lower an AST into an executable [`Plan`].
pub fn plan(input: &str) -> Result<Plan, QueryError> {
    plan::lower(&parse(input)?)
}

/// Evaluate a plan against a store at instant `eval`.
pub fn evaluate<S: Storage>(plan: &Plan, store: &S, eval: Timestamp) -> Result<Value, QueryError> {
    executor::evaluate(plan, store, eval)
}

/// Parse, plan, and evaluate a query against a store in one call.
pub fn run<S: Storage>(input: &str, store: &S, eval: Timestamp) -> Result<Value, QueryError> {
    evaluate(&plan(input)?, store, eval)
}
