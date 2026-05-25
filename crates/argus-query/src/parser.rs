//! Turns tokens into an AST (recursive descent + precedence climbing).

use crate::ast::{AggOp, BinOp, Expr, Grouping, Matcher, Selector};
use crate::error::QueryError;
use crate::token::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Expr, QueryError> {
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.expr(0)?;
    parser.expect(Token::Eof)?;
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn next(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        token
    }

    fn expect(&mut self, expected: Token) -> Result<(), QueryError> {
        let actual = self.next();
        if actual == expected {
            Ok(())
        } else {
            Err(QueryError::Parse(format!(
                "expected {expected:?}, found {actual:?}"
            )))
        }
    }

    /// Precedence-climbing binary expression parser.
    fn expr(&mut self, min_bp: u8) -> Result<Expr, QueryError> {
        let mut lhs = self.unary()?;
        while let Some((op, bp)) = binary_op(self.peek()) {
            if bp < min_bp {
                break;
            }
            self.next();
            let rhs = self.expr(bp + 1)?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, QueryError> {
        if *self.peek() == Token::Minus {
            self.next();
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, QueryError> {
        match self.next() {
            Token::Number(value) => Ok(Expr::Number(value)),
            Token::LParen => {
                let inner = self.expr(0)?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            Token::Ident(name) => {
                if let Some(op) = AggOp::from_name(&name) {
                    self.aggregation(op)
                } else if *self.peek() == Token::LParen {
                    self.call(name)
                } else {
                    self.selector(name)
                }
            }
            other => Err(QueryError::Parse(format!("unexpected token {other:?}"))),
        }
    }

    fn call(&mut self, func: String) -> Result<Expr, QueryError> {
        self.expect(Token::LParen)?;
        let args = self.arg_list()?;
        Ok(Expr::Call { func, args })
    }

    fn aggregation(&mut self, op: AggOp) -> Result<Expr, QueryError> {
        let grouping = self.optional_grouping()?;
        self.expect(Token::LParen)?;
        let mut args = self.arg_list()?;

        let (param, arg) = if op == AggOp::Quantile {
            if args.len() != 2 {
                return Err(QueryError::Parse(
                    "quantile expects (scalar, vector)".into(),
                ));
            }
            let arg = args.pop().expect("len checked");
            let param = args.pop().expect("len checked");
            (Some(Box::new(param)), arg)
        } else {
            if args.len() != 1 {
                return Err(QueryError::Parse(format!(
                    "{} expects one argument",
                    op.name()
                )));
            }
            (None, args.pop().expect("len checked"))
        };

        Ok(Expr::Aggregate {
            op,
            grouping,
            param,
            arg: Box::new(arg),
        })
    }

    fn optional_grouping(&mut self) -> Result<Grouping, QueryError> {
        let keyword = match self.peek() {
            Token::Ident(name) if name == "by" || name == "without" => name.clone(),
            _ => return Ok(Grouping::None),
        };
        self.next();
        let labels = self.label_list()?;
        Ok(if keyword == "by" {
            Grouping::By(labels)
        } else {
            Grouping::Without(labels)
        })
    }

    fn label_list(&mut self) -> Result<Vec<String>, QueryError> {
        self.expect(Token::LParen)?;
        let mut labels = Vec::new();
        while *self.peek() != Token::RParen {
            match self.next() {
                Token::Ident(name) => labels.push(name),
                other => {
                    return Err(QueryError::Parse(format!(
                        "expected label name, found {other:?}"
                    )));
                }
            }
            if *self.peek() == Token::Comma {
                self.next();
            }
        }
        self.expect(Token::RParen)?;
        Ok(labels)
    }

    fn arg_list(&mut self) -> Result<Vec<Expr>, QueryError> {
        let mut args = Vec::new();
        while *self.peek() != Token::RParen {
            args.push(self.expr(0)?);
            if *self.peek() == Token::Comma {
                self.next();
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;
        Ok(args)
    }

    fn selector(&mut self, metric: String) -> Result<Expr, QueryError> {
        let mut matchers = Vec::new();
        if *self.peek() == Token::LBrace {
            self.next();
            matchers = self.matchers()?;
        }

        let mut range_ms = None;
        if *self.peek() == Token::LBracket {
            self.next();
            range_ms = Some(self.duration()?);
            self.expect(Token::RBracket)?;
        }

        Ok(Expr::Selector(Selector {
            metric,
            matchers,
            range_ms,
        }))
    }

    fn matchers(&mut self) -> Result<Vec<Matcher>, QueryError> {
        let mut matchers = Vec::new();
        while *self.peek() != Token::RBrace {
            let label = match self.next() {
                Token::Ident(name) => name,
                other => {
                    return Err(QueryError::Parse(format!(
                        "expected label, found {other:?}"
                    )));
                }
            };
            let op = self.next();
            let value = match self.next() {
                Token::Str(value) => value,
                other => {
                    return Err(QueryError::Parse(format!(
                        "expected string value, found {other:?}"
                    )));
                }
            };
            matchers.push(match op {
                Token::Eq => Matcher::Eq(label, value),
                Token::Ne => Matcher::Ne(label, value),
                Token::EqRegex => Matcher::ReEq(label, value),
                Token::NeRegex => Matcher::ReNe(label, value),
                other => {
                    return Err(QueryError::Parse(format!(
                        "expected matcher operator, found {other:?}"
                    )));
                }
            });
            if *self.peek() == Token::Comma {
                self.next();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(matchers)
    }

    fn duration(&mut self) -> Result<u64, QueryError> {
        match self.next() {
            Token::Duration(ms) => Ok(ms),
            other => Err(QueryError::Parse(format!(
                "expected a duration like [5m], found {other:?}"
            ))),
        }
    }
}

fn binary_op(token: &Token) -> Option<(BinOp, u8)> {
    match token {
        Token::Plus => Some((BinOp::Add, 10)),
        Token::Minus => Some((BinOp::Sub, 10)),
        Token::Star => Some((BinOp::Mul, 20)),
        Token::Slash => Some((BinOp::Div, 20)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_str(input: &str) -> Result<Expr, QueryError> {
        parse(lex(input).unwrap())
    }

    #[test]
    fn parses_aggregation_over_range_function() {
        let expr = parse_str("sum by (status) (rate(http_requests_total[5m]))").unwrap();
        match expr {
            Expr::Aggregate {
                op: AggOp::Sum,
                grouping: Grouping::By(labels),
                arg,
                param: None,
            } => {
                assert_eq!(labels, vec!["status".to_owned()]);
                assert!(matches!(*arg, Expr::Call { .. }));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn respects_arithmetic_precedence() {
        // 1 + 2 * 3  parses as  1 + (2 * 3)
        let expr = parse_str("1 + 2 * 3").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Add,
                rhs,
                ..
            } => {
                assert!(matches!(*rhs, Expr::Binary { op: BinOp::Mul, .. }));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_quantile_with_param() {
        let expr = parse_str("quantile(0.95, http_request_duration_ms)").unwrap();
        assert!(matches!(
            expr,
            Expr::Aggregate {
                op: AggOp::Quantile,
                param: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse_str("foo bar").is_err());
    }
}
