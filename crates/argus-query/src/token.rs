//! Lexical tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A metric name, label key, function, or keyword (`by`/`without`).
    Ident(String),
    Number(f64),
    /// A quoted label value.
    Str(String),
    /// A duration literal, normalized to milliseconds (`5m` → `300000`).
    Duration(u64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Eq,
    Ne,
    EqRegex,
    NeRegex,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}
