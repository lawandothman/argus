//! Turns a query string into a flat token stream.

use crate::error::QueryError;
use crate::token::Token;

pub fn lex(input: &str) -> Result<Vec<Token>, QueryError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        let ch = chars[pos];

        if ch.is_whitespace() {
            pos += 1;
            continue;
        }

        match ch {
            '(' => push(&mut tokens, Token::LParen, &mut pos),
            ')' => push(&mut tokens, Token::RParen, &mut pos),
            '{' => push(&mut tokens, Token::LBrace, &mut pos),
            '}' => push(&mut tokens, Token::RBrace, &mut pos),
            '[' => push(&mut tokens, Token::LBracket, &mut pos),
            ']' => push(&mut tokens, Token::RBracket, &mut pos),
            ',' => push(&mut tokens, Token::Comma, &mut pos),
            '+' => push(&mut tokens, Token::Plus, &mut pos),
            '-' => push(&mut tokens, Token::Minus, &mut pos),
            '*' => push(&mut tokens, Token::Star, &mut pos),
            '/' => push(&mut tokens, Token::Slash, &mut pos),
            '=' => {
                if chars.get(pos + 1) == Some(&'~') {
                    push(&mut tokens, Token::EqRegex, &mut pos);
                    pos += 1;
                } else {
                    push(&mut tokens, Token::Eq, &mut pos);
                }
            }
            '!' => match chars.get(pos + 1) {
                Some('=') => {
                    tokens.push(Token::Ne);
                    pos += 2;
                }
                Some('~') => {
                    tokens.push(Token::NeRegex);
                    pos += 2;
                }
                _ => return Err(QueryError::Lex("expected '=' or '~' after '!'".into())),
            },
            '"' | '\'' => {
                let (string, next) = lex_string(&chars, pos)?;
                tokens.push(Token::Str(string));
                pos = next;
            }
            c if c.is_ascii_digit() => {
                let (token, next) = lex_number(&chars, pos);
                tokens.push(token);
                pos = next;
            }
            c if is_ident_start(c) => {
                let (ident, next) = lex_ident(&chars, pos);
                tokens.push(Token::Ident(ident));
                pos = next;
            }
            other => return Err(QueryError::Lex(format!("unexpected character '{other}'"))),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

fn push(tokens: &mut Vec<Token>, token: Token, pos: &mut usize) {
    tokens.push(token);
    *pos += 1;
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == ':'
}

fn is_ident_part(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == ':'
}

fn lex_ident(chars: &[char], start: usize) -> (String, usize) {
    let mut pos = start;
    while pos < chars.len() && is_ident_part(chars[pos]) {
        pos += 1;
    }
    (chars[start..pos].iter().collect(), pos)
}

fn lex_string(chars: &[char], start: usize) -> Result<(String, usize), QueryError> {
    let quote = chars[start];
    let mut pos = start + 1;
    let mut out = String::new();
    while pos < chars.len() {
        match chars[pos] {
            c if c == quote => return Ok((out, pos + 1)),
            '\\' if pos + 1 < chars.len() => {
                out.push(chars[pos + 1]);
                pos += 2;
            }
            c => {
                out.push(c);
                pos += 1;
            }
        }
    }
    Err(QueryError::Lex("unterminated string".into()))
}

/// Lex a number, or a duration when an integer is immediately followed by a
/// unit (`ms`, `s`, `m`, `h`, `d`, `w`).
fn lex_number(chars: &[char], start: usize) -> (Token, usize) {
    let mut pos = start;
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        pos += 1;
    }
    let is_integer = pos >= chars.len() || chars[pos] != '.';

    if is_integer {
        if let Some((ms_per_unit, unit_len)) = duration_unit(chars, pos) {
            let count: u64 = chars[start..pos]
                .iter()
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            return (Token::Duration(count * ms_per_unit), pos + unit_len);
        }
    } else {
        // consume the fractional part
        pos += 1;
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            pos += 1;
        }
    }

    let value: f64 = chars[start..pos]
        .iter()
        .collect::<String>()
        .parse()
        .unwrap_or(0.0);
    (Token::Number(value), pos)
}

fn duration_unit(chars: &[char], pos: usize) -> Option<(u64, usize)> {
    let at = |offset: usize| chars.get(pos + offset).copied();
    match (at(0), at(1)) {
        (Some('m'), Some('s')) => Some((1, 2)),
        (Some('s'), _) => Some((1_000, 1)),
        (Some('m'), _) => Some((60_000, 1)),
        (Some('h'), _) => Some((3_600_000, 1)),
        (Some('d'), _) => Some((86_400_000, 1)),
        (Some('w'), _) => Some((604_800_000, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_a_full_query() {
        let tokens = lex("sum by (status) (rate(http_requests_total[5m]))").unwrap();
        assert_eq!(tokens[0], Token::Ident("sum".into()));
        assert!(tokens.contains(&Token::Duration(300_000)));
        assert_eq!(tokens.last(), Some(&Token::Eof));
    }

    #[test]
    fn distinguishes_numbers_and_durations() {
        assert_eq!(lex("0.95").unwrap()[0], Token::Number(0.95));
        assert_eq!(lex("500ms").unwrap()[0], Token::Duration(500));
        assert_eq!(lex("2h").unwrap()[0], Token::Duration(7_200_000));
    }

    #[test]
    fn lexes_matchers() {
        let tokens = lex(r#"x{a="b",c!="d"}"#).unwrap();
        assert!(tokens.contains(&Token::Eq));
        assert!(tokens.contains(&Token::Ne));
        assert!(tokens.contains(&Token::Str("b".into())));
    }
}
