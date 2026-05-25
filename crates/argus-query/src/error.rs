//! Query errors, surfaced with the stage that produced them.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    Lex(String),
    Parse(String),
    /// A construct that parses but the engine does not yet support.
    Unsupported(String),
    Eval(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::Lex(message) => write!(f, "lex error: {message}"),
            QueryError::Parse(message) => write!(f, "parse error: {message}"),
            QueryError::Unsupported(message) => write!(f, "unsupported: {message}"),
            QueryError::Eval(message) => write!(f, "evaluation error: {message}"),
        }
    }
}

impl std::error::Error for QueryError {}
