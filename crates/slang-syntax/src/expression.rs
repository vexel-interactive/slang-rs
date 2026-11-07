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
            ExprKind::Unary { value, .. }
            | ExprKind::Postfix { value, .. }
            | ExprKind::Cast { value, .. } => value.height,
            ExprKind::Binary { left, right, .. } => left.height.max(right.height),
            ExprKind::Call { callee, arguments } => callee
                .height
                .max(arguments.iter().map(|a| a.height).max().unwrap_or(0)),
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
            height,
        })
    }

    fn expression_inner(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        let token = self
            .current()
            .ok_or_else(|| self.error("expected expression"))?
            .clone();
        self.offset += 1;
        let mut left = if token.is("(") && self.cast_type() {
            self.cast_expression(token.span)?
        } else if token.is("(") {
            let mut expr = self.expression(0)?;
            let end = self.expect(")")?;
            expr.span = token.span.join(end);
            expr
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
            if self.eat(".") {
                let name = self.identifier()?;
                let span = left.span.join(name.span);
                left = self.expr(
                    ExprKind::Member {
                        base: Box::new(left),
                        name: name.text,
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
            let precedence = match token.text.as_str() {
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
        let Some(token) = self.current() else {
            return false;
        };
        self.tokens.get(self.offset + 1).is_some_and(|t| t.is(")"))
            && builtin_cast_type(&token.text)
    }

    fn cast_expression(&mut self, start: Span) -> Result<Expr, Diagnostic> {
        let target = self.ty()?;
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
    matches!(name, "bool" | "int" | "uint" | "half" | "float" | "double")
}
