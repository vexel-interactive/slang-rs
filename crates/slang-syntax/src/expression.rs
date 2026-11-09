use crate::{Expr, ExprId, ExprKind, parser::Parser};
use slang_lexer::TokenKind;
use slang_source::{Diagnostic, Span};

impl Parser {
    pub(crate) fn expression(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        self.enter()?;
        let result = self.expression_inner(min);
        self.depth -= 1;
        result
    }

    fn expr(&mut self, kind: ExprKind, span: Span) -> Result<Expr, Diagnostic> {
        let height = 1 + match &kind {
            ExprKind::InitializerList(values) => values.iter().map(|v| v.height).max().unwrap_or(0),
            ExprKind::Unary { value, .. }
            | ExprKind::Postfix { value, .. }
            | ExprKind::Cast { value, .. } => value.height,
            ExprKind::Binary { left, right, .. } => left.height.max(right.height),
            ExprKind::Call { callee, arguments } => callee
                .height
                .max(arguments.iter().map(|a| a.height).max().unwrap_or(0)),
            ExprKind::GenericApplication { callee, .. } => callee.height,
            ExprKind::Member { base, .. } => base.height,
            ExprKind::Index { base, index } => base.height.max(index.height),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => condition
                .height
                .max(then_value.height)
                .max(else_value.height),
            _ => 0,
        };
        if height > 64 {
            return Err(Diagnostic::error(
                "S001",
                span,
                "expression tree depth limit exceeded",
            ));
        }
        let id = ExprId(self.next_expr);
        self.next_expr += 1;
        Ok(Expr {
            id,
            kind,
            span,
            parenthesized: false,
            height,
        })
    }

    fn expression_inner(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        let token = self
            .current()
            .ok_or_else(|| self.error("expected expression"))?
            .clone();
        self.offset += 1;
        let mut left = if token.is("{") {
            let mut values = vec![];
            if !self.at("}") {
                loop {
                    values.push(self.expression(1)?);
                    if !self.eat(",") || self.at("}") {
                        break;
                    }
                }
            }
            let end = self.expect("}")?;
            self.expr(ExprKind::InitializerList(values), token.span.join(end))?
        } else if token.is("(") && self.cast_type() {
            self.cast_expression(token.span)?
        } else if token.is("(") {
            let mut expr = self.expression(0)?;
            let end = self.expect(")")?;
            expr.span = token.span.join(end);
            expr.parenthesized = true;
            expr
        } else if token.kind == TokenKind::Identifier
            && self
                .type_application_end(self.offset - 1)
                .is_some_and(|end| {
                    self.tokens
                        .get(end)
                        .is_some_and(|t| t.is("(") || t.is("[") || t.is(".") || t.is("::"))
                })
        {
            self.offset -= 1;
            let target = self.expression_type()?;
            let span = target.span;
            self.expr(ExprKind::TypeApplication(Box::new(target)), span)?
        } else if matches!(token.text.as_str(), "+" | "-" | "!" | "~" | "++" | "--") {
            let value = self.expression(14)?;
            let span = token.span.join(value.span);
            self.expr(
                ExprKind::Unary {
                    op: token.text,
                    value: Box::new(value),
                },
                span,
            )?
        } else {
            let kind = match token.kind {
                TokenKind::Integer => ExprKind::Integer(token.text),
                TokenKind::Float => ExprKind::Float(token.text),
                TokenKind::Character => {
                    let value =
                        slang_lexer::parse_character_literal(&token.text).map_err(|error| {
                            Diagnostic::error(
                                "L006",
                                token.span,
                                match error {
                                    slang_lexer::CharacterLiteralError::CharacterCount => {
                                        "character literal must contain exactly one value"
                                    }
                                    slang_lexer::CharacterLiteralError::Escape => {
                                        "invalid escape in character literal"
                                    }
                                    slang_lexer::CharacterLiteralError::Overflow => {
                                        "character escape exceeds 32 bits"
                                    }
                                },
                            )
                        })?;
                    ExprKind::Integer(format!("{value}u"))
                }
                TokenKind::String => ExprKind::String(token.text),
                TokenKind::Identifier if token.is("true") || token.is("false") => {
                    ExprKind::Bool(token.is("true"))
                }
                TokenKind::Identifier => ExprKind::Name(token.text),
                _ => return Err(Diagnostic::error("S001", token.span, "expected expression")),
            };
            self.expr(kind, token.span)?
        };
        loop {
            if self.at("(") && min <= 15 {
                self.offset += 1;
                let arguments = self.arguments(")")?;
                let span = left.span.join(self.tokens[self.offset - 1].span);
                left = self.expr(
                    ExprKind::Call {
                        callee: Box::new(left),
                        arguments,
                    },
                    span,
                )?;
                continue;
            }
            if self.at("<")
                && matches!(left.kind, ExprKind::Member { .. })
                && self
                    .type_application_end(self.offset - 1)
                    .is_some_and(|end| {
                        self.tokens
                            .get(end)
                            .is_some_and(|t| t.is("(") || t.is(".") || t.is("::"))
                    })
            {
                let ExprKind::Member { name, .. } = &left.kind else {
                    unreachable!()
                };
                let arguments = self.type_arguments(name)?;
                let span = left.span.join(self.tokens[self.offset - 1].span);
                left = self.expr(
                    ExprKind::GenericApplication {
                        callee: Box::new(left),
                        arguments,
                    },
                    span,
                )?;
                continue;
            }
            if self.eat("[") {
                let index = self.expression(0)?;
                let end = self.expect("]")?;
                let span = left.span.join(end);
                left = self.expr(
                    ExprKind::Index {
                        base: Box::new(left),
                        index: Box::new(index),
                    },
                    span,
                )?;
                continue;
            }
            if self.at(".") || self.at("::") {
                let static_lookup = self.eat("::");
                if !static_lookup {
                    self.expect(".")?;
                }
                let name = self.identifier()?;
                let span = left.span.join(name.span);
                left = self.expr(
                    ExprKind::Member {
                        base: Box::new(left),
                        name: name.text,
                        static_lookup,
                    },
                    span,
                )?;
                continue;
            }
            if self.at("++") || self.at("--") {
                let token = self.current().unwrap().clone();
                self.offset += 1;
                let span = left.span.join(token.span);
                left = self.expr(
                    ExprKind::Postfix {
                        op: token.text,
                        value: Box::new(left),
                    },
                    span,
                )?;
                continue;
            }
            if self.at("?") && min <= 2 {
                self.offset += 1;
                let then_value = self.expression(0)?;
                self.expect(":")?;
                let else_value = self.expression(2)?;
                let span = left.span.join(else_value.span);
                left = self.expr(
                    ExprKind::Conditional {
                        condition: Box::new(left),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                    span,
                )?;
                continue;
            }
            let Some(token) = self.current() else {
                break;
            };
            if self.expression_angle_stop && matches!(token.text.as_str(), ">" | ">>" | ">=") {
                break;
            }
            let precedence = match token.text.as_str() {
                "," => 0,
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" => 1,
                "||" => 3,
                "&&" => 4,
                "|" => 5,
                "^" => 6,
                "&" => 7,
                "==" | "!=" => 8,
                "<" | "<=" | ">" | ">=" => 9,
                "<<" | ">>" => 10,
                "+" | "-" => 11,
                "*" | "/" | "%" => 12,
                _ => break,
            };
            if precedence < min {
                break;
            }
            let op = token.text.clone();
            self.offset += 1;
            let right = self.expression(if precedence == 1 { 1 } else { precedence + 1 })?;
            let span = left.span.join(right.span);
            left = self.expr(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )?;
        }
        Ok(left)
    }

    fn cast_type(&self) -> bool {
        let mut start = self.offset;
        while self
            .tokens
            .get(start)
            .is_some_and(|t| matches!(t.text.as_str(), "row_major" | "column_major"))
        {
            start += 1;
        }
        let Some(token) = self.tokens.get(start) else {
            return false;
        };
        if matches!(token.text.as_str(), "vector" | "matrix") {
            return self
                .type_application_end(start)
                .is_some_and(|end| self.tokens.get(end).is_some_and(|t| t.is(")")));
        }
        self.tokens.get(start + 1).is_some_and(|t| t.is(")"))
            && (builtin_cast_type(&token.text)
                || self
                    .type_parameters
                    .iter()
                    .rev()
                    .any(|p| p.contains(&token.text)))
    }

    /// Recognize angle brackets without consuming comparisons or shifting tokens.
    pub(crate) fn type_application_end(&self, start: usize) -> Option<usize> {
        if !self.tokens.get(start + 1)?.is("<") {
            return None;
        }
        let mut applications: Vec<(bool, usize)> = vec![];
        let mut parentheses = 0usize;
        for (offset, token) in self.tokens.iter().enumerate().skip(start + 1) {
            if token.is("(") {
                parentheses += 1;
                continue;
            }
            if token.is(")") {
                if parentheses == 0 {
                    return None;
                }
                parentheses -= 1;
                continue;
            }
            if parentheses > 0 {
                continue;
            }
            match token.text.as_str() {
                "<" if applications
                    .last()
                    .is_some_and(|&(numeric, index)| numeric && index > 0) => {}
                "<" => applications.push((
                    self.tokens
                        .get(offset - 1)
                        .is_some_and(|t| matches!(t.text.as_str(), "vector" | "matrix")),
                    0,
                )),
                "," => applications.last_mut()?.1 += 1,
                ">" => {
                    applications.pop()?;
                }
                ">>" => {
                    applications.pop()?;
                    applications.pop()?;
                }
                ";" | "{" | "}" | "=" => return None,
                _ => {}
            }
            if applications.is_empty() {
                return Some(offset + 1);
            }
            if token.kind == TokenKind::Eof {
                return None;
            }
        }
        None
    }

    fn cast_expression(&mut self, start: Span) -> Result<Expr, Diagnostic> {
        let mut target = self.ty()?;
        target.parameter_ref = self
            .type_parameters
            .iter()
            .rev()
            .any(|p| p.contains(&target.name));
        self.expect(")")?;
        let value = self.expression(14)?;
        let span = start.join(value.span);
        self.expr(
            ExprKind::Cast {
                target: Box::new(target),
                value: Box::new(value),
            },
            span,
        )
    }
}

fn builtin_cast_type(name: &str) -> bool {
    if matches!(
        name,
        "bool"
            | "int"
            | "uint"
            | "half"
            | "float"
            | "double"
            | "int32_t"
            | "uint32_t"
            | "int64_t"
            | "uint64_t"
            | "float32_t"
            | "float64_t"
    ) {
        return true;
    }
    [
        "bool", "int", "uint", "int64_t", "uint64_t", "half", "float", "double",
    ]
    .iter()
    .any(|prefix| {
        let Some(shape) = name.strip_prefix(prefix) else {
            return false;
        };
        let dimension = |text: &str| text.parse::<u8>().is_ok_and(|n| (1..=4).contains(&n));
        shape
            .split_once('x')
            .map_or_else(|| dimension(shape), |(r, c)| dimension(r) && dimension(c))
    })
}
