//! The abstract syntax tree produced by the parser.

/// A query expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Selector(Selector),
    Call {
        func: String,
        args: Vec<Expr>,
    },
    Aggregate {
        op: AggOp,
        grouping: Grouping,
        param: Option<Box<Expr>>,
        arg: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Neg(Box<Expr>),
}

/// A metric selector, optionally with a range (`[5m]`) making it a range vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub metric: String,
    pub matchers: Vec<Matcher>,
    pub range_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    Eq(String, String),
    Ne(String, String),
    ReEq(String, String),
    ReNe(String, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggOp {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    Quantile,
}

impl AggOp {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "sum" => Some(AggOp::Sum),
            "avg" => Some(AggOp::Avg),
            "min" => Some(AggOp::Min),
            "max" => Some(AggOp::Max),
            "count" => Some(AggOp::Count),
            "quantile" => Some(AggOp::Quantile),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AggOp::Sum => "sum",
            AggOp::Avg => "avg",
            AggOp::Min => "min",
            AggOp::Max => "max",
            AggOp::Count => "count",
            AggOp::Quantile => "quantile",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grouping {
    None,
    By(Vec<String>),
    Without(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
        }
    }

    pub fn apply(self, lhs: f64, rhs: f64) -> f64 {
        match self {
            BinOp::Add => lhs + rhs,
            BinOp::Sub => lhs - rhs,
            BinOp::Mul => lhs * rhs,
            BinOp::Div => lhs / rhs,
        }
    }
}
