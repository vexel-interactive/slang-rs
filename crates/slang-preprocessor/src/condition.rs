use slang_lexer::{Token, TokenKind};
use slang_source::{Diagnostic, Span};

pub(crate) fn evaluate(tokens: &[Token]) -> Result<i64, Diagnostic> {
    let mut parser = Evaluator {
        tokens,
        offset: 0,
        depth: 0,
    };
    let result = parser.expr(0, true)?;
    if parser.offset != tokens.len() {
        return Err(parser.error("unexpected tokens in #if expression"));
    }
    Ok(result)
}

struct Evaluator<'a> {
    tokens: &'a [Token],
    offset: usize,
    depth: usize,
}

impl Evaluator<'_> {
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic::error(
            "P007",
            self.tokens
                .get(self.offset)
                .or_else(|| self.tokens.last())
                .map_or(Span::default(), |t| t.span),
            message,
        )
    }

    fn expr(&mut self, min: u8, execute: bool) -> Result<i64, Diagnostic> {
        if self.depth >= 128 {
            return Err(self.error("#if expression nesting limit exceeded"));
        }
        self.depth += 1;
        let result = self.expr_inner(min, execute);
        self.depth -= 1;
        result
    }

    fn expr_inner(&mut self, min: u8, execute: bool) -> Result<i64, Diagnostic> {
        let token = self
            .tokens
            .get(self.offset)
            .ok_or_else(|| self.error("expected #if expression"))?;
        self.offset += 1;
        let mut left = match token.text.as_str() {
            "(" => {
                let value = self.expr(0, execute)?;
                if !self.tokens.get(self.offset).is_some_and(|t| t.is(")")) {
                    return Err(self.error("expected ')'"));
                }
                self.offset += 1;
                value
            }
            "!" => i64::from(self.expr(12, execute)? == 0),
            "~" => !self.expr(12, execute)?,
            "-" => self.expr(12, execute)?.wrapping_neg(),
            "+" => self.expr(12, execute)?,
            _ if token.kind == TokenKind::Identifier => 0,
            _ if token.kind == TokenKind::Integer => integer(&token.text)
                .ok_or_else(|| self.error("integer literal out of range in #if"))?,
            _ => return Err(self.error("expected integer expression")),
        };
        while let Some(token) = self.tokens.get(self.offset) {
            let precedence = match token.text.as_str() {
                "||" => 1,
                "&&" => 2,
                "|" => 3,
                "^" => 4,
                "&" => 5,
                "==" | "!=" => 6,
                "<" | "<=" | ">" | ">=" => 7,
                "<<" | ">>" => 8,
                "+" | "-" => 9,
                "*" | "/" | "%" => 10,
                _ => break,
            };
            if precedence < min {
                break;
            }
            let op = token.text.as_str();
            self.offset += 1;
            let rhs_execute = execute && !(op == "&&" && left == 0 || op == "||" && left != 0);
            let right = self.expr(precedence + 1, rhs_execute)?;
            if !execute {
                left = 0;
                continue;
            }
            left = match op {
                "||" => i64::from(left != 0 || right != 0),
                "&&" => i64::from(left != 0 && right != 0),
                "|" => left | right,
                "^" => left ^ right,
                "&" => left & right,
                "==" => i64::from(left == right),
                "!=" => i64::from(left != right),
                "<" => i64::from(left < right),
                "<=" => i64::from(left <= right),
                ">" => i64::from(left > right),
                ">=" => i64::from(left >= right),
                "<<" | ">>" => {
                    if !(0..64).contains(&right) {
                        return Err(self.error("invalid shift amount in #if"));
                    }
                    if op == "<<" {
                        left.wrapping_shl(right as u32)
                    } else {
                        left.wrapping_shr(right as u32)
                    }
                }
                "+" => left.wrapping_add(right),
                "-" => left.wrapping_sub(right),
                "*" => left.wrapping_mul(right),
                "/" | "%" => {
                    if right == 0 {
                        return Err(self.error("division by zero in #if"));
                    }
                    if op == "/" {
                        left.wrapping_div(right)
                    } else {
                        left.wrapping_rem(right)
                    }
                }
                _ => unreachable!(),
            };
        }
        Ok(left)
    }
}

fn integer(text: &str) -> Option<i64> {
    let text = text.trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(text) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(text, 16).ok()
    } else if let Some(text) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        i64::from_str_radix(text, 2).ok()
    } else {
        text.parse().ok()
    }
}
