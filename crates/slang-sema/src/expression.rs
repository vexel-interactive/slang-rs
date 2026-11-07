use crate::{Scalar, TextureDimension, Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Expr, ExprKind};

impl Checker {
    pub fn expression(&mut self, expression: &Expr) -> Type {
        let ty = match &expression.kind {
            ExprKind::Integer(text) => {
                let digits = text.trim_end_matches(['u', 'U', 'l', 'L']);
                let value = if let Some(hex) = digits
                    .strip_prefix("0x")
                    .or_else(|| digits.strip_prefix("0X"))
                {
                    u64::from_str_radix(hex, 16).ok()
                } else if let Some(binary) = digits
                    .strip_prefix("0b")
                    .or_else(|| digits.strip_prefix("0B"))
                {
                    u64::from_str_radix(binary, 2).ok()
                } else {
                    digits.parse::<u64>().ok()
                };
                if text.contains(['l', 'L']) {
                    self.error(
                        "T001",
                        expression.span,
                        "64-bit integer literals are not implemented",
                    );
                }
                let unsigned =
                    text.contains(['u', 'U']) || value.is_some_and(|v| v > i32::MAX as u64);
                if value.is_none_or(|v| v > u32::MAX as u64) {
                    self.error("T010", expression.span, "integer literal exceeds 32 bits");
                }
                Type::Scalar(if unsigned { Scalar::Uint } else { Scalar::Int })
            }
            ExprKind::Float(text) => {
                let value = text
                    .trim_end_matches(['f', 'F', 'h', 'H'])
                    .parse::<f64>()
                    .ok();
                if value.is_none_or(|v| !v.is_finite() || v.abs() > f32::MAX as f64) {
                    self.error(
                        "T010",
                        expression.span,
                        "floating-point literal is out of range",
                    );
                }
                Type::Scalar(if text.ends_with(['h', 'H']) {
                    Scalar::Half
                } else {
                    Scalar::Float
                })
            }
            ExprKind::Bool(_) => Type::Scalar(Scalar::Bool),
            ExprKind::Cast { target, value } => {
                let actual = self.expression(value);
                let target = self.resolve_type(target);
                let valid = actual == Type::Error
                    || target == Type::Error
                    || matches!(actual, Type::Scalar(_)) && matches!(target, Type::Scalar(_));
                if !valid {
                    self.error(
                        "T008",
                        expression.span,
                        format!("unsupported explicit cast from {actual} to {target}"),
                    );
                }
                target
            }
            ExprKind::String(_) => {
                self.error(
                    "T001",
                    expression.span,
                    "string values are only implemented in attributes",
                );
                Type::Error
            }
            ExprKind::Name(name) => {
                if let Some(binding) = self.lookup(name) {
                    binding.ty.clone()
                } else {
                    self.error("T011", expression.span, format!("unknown value '{name}'"));
                    Type::Error
                }
            }
            ExprKind::Unary { op, value } => {
                let ty = self.expression(value);
                match op.as_str() {
                    "!" if ty.scalar().is_some() => ty.boolean_shape(),
                    "~" if ty.integral() => ty.clone(),
                    "+" | "-" if ty.numeric() => ty.clone(),
                    "++" | "--" if ty.numeric() => {
                        self.require_writable(value);
                        ty.clone()
                    }
                    _ if ty == Type::Error => Type::Error,
                    _ => {
                        self.error(
                            "T012",
                            expression.span,
                            format!("operator {op} is not defined for {ty}"),
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Postfix { value, .. } => {
                let ty = self.expression(value);
                self.require_writable(value);
                if !ty.numeric() && ty != Type::Error {
                    self.error(
                        "T012",
                        expression.span,
                        "increment requires a numeric value",
                    );
                }
                ty
            }
            ExprKind::Binary { op, left, right } => {
                let a = self.expression(left);
                let b = self.expression(right);
                if op == "=" {
                    self.require_writable(left);
                    self.require_conversion(&b, &a, right.span);
                    a
                } else if matches!(
                    op.as_str(),
                    "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                ) {
                    self.require_writable(left);
                    let result = self.binary(op.trim_end_matches('='), &a, &b, expression.span);
                    self.require_conversion(&result, &a, expression.span);
                    a
                } else {
                    self.binary(op, &a, &b, expression.span)
                }
            }
            ExprKind::Call { callee, arguments } => self.call(callee, arguments, expression.span),
            ExprKind::Member { base, name } => {
                let ty = self.expression(base);
                match ty {
                    Type::Struct(ref structure)
                        if self
                            .structures
                            .get(structure)
                            .is_some_and(|fields| fields.contains_key(name))
                            && !self.field_accessible(structure, name) =>
                    {
                        self.error(
                            "T018",
                            expression.span,
                            format!("field '{name}' is internal to its module"),
                        );
                        Type::Error
                    }
                    Type::Struct(structure) => self
                        .structures
                        .get(&structure)
                        .and_then(|s| s.get(name))
                        .cloned()
                        .unwrap_or_else(|| {
                            self.error("T013", expression.span, format!("unknown field '{name}'"));
                            Type::Error
                        }),
                    Type::Vector(scalar, width) => {
                        if let Some(indices) =
                            swizzle(name).filter(|v| v.len() <= 4 && v.iter().all(|&i| i < width))
                        {
                            if indices.len() == 1 {
                                Type::Scalar(scalar)
                            } else {
                                Type::Vector(scalar, indices.len() as u8)
                            }
                        } else {
                            self.error(
                                "T013",
                                expression.span,
                                format!("invalid swizzle '{name}'"),
                            );
                            Type::Error
                        }
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(
                            "T013",
                            expression.span,
                            format!("{ty} has no field '{name}'"),
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Index { base, index } => {
                let ty = self.expression(base);
                let index_type = self.expression(index);
                let width = match &ty {
                    Type::Texture { dimension, .. } => dimension.coordinates(),
                    _ => 1,
                };
                if index_type != Type::Error
                    && !(index_type.integral() && index_type.width() == Some(width))
                {
                    self.error(
                        "T013",
                        index.span,
                        format!("index requires an integer with {width} component(s)"),
                    );
                }
                let (result, size) = match ty {
                    Type::Array(element, size) => (*element, Some(size)),
                    Type::Vector(s, n) => (Type::Scalar(s), Some(u32::from(n))),
                    Type::Matrix(s, _, columns) => (Type::Vector(s, columns), None),
                    Type::Buffer { element, .. } => (*element, None),
                    Type::Texture {
                        element, dimension, ..
                    } if dimension != TextureDimension::Cube => (*element, None),
                    Type::Error => (Type::Error, None),
                    _ => {
                        self.error("T013", expression.span, format!("cannot index {ty}"));
                        (Type::Error, None)
                    }
                };
                if let Some(size) = size
                    && let ExprKind::Integer(text) = &index.kind
                    && text.parse::<u32>().is_ok_and(|v| v >= size)
                {
                    self.error("T013", index.span, "constant index is out of bounds");
                }
                result
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.condition(condition);
                let a = self.expression(then_value);
                let b = self.expression(else_value);
                if a == b {
                    a
                } else {
                    Type::common(&a, &b).unwrap_or_else(|| {
                        self.error("T008", expression.span, "incompatible conditional branches");
                        Type::Error
                    })
                }
            }
        };
        self.expression_types.insert(expression.id, ty.clone());
        ty
    }

    fn binary(&mut self, op: &str, a: &Type, b: &Type, span: Span) -> Type {
        if *a == Type::Error || *b == Type::Error {
            return Type::Error;
        }
        let common = Type::common(a, b);
        let valid = match op {
            "&&" | "||" => matches!(a, Type::Scalar(_)) && matches!(b, Type::Scalar(_)),
            "==" | "!=" => common.is_some(),
            "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" => {
                a.numeric() && b.numeric() && common.is_some()
            }
            "%" | "&" | "|" | "^" | "<<" | ">>" => a.integral() && b.integral() && common.is_some(),
            _ => false,
        };
        if !valid {
            self.error(
                "T012",
                span,
                format!("operator {op} is not defined for {a} and {b}"),
            );
            return Type::Error;
        }
        if matches!(op, "&&" | "||") {
            Type::Scalar(Scalar::Bool)
        } else if matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=") {
            common.unwrap().boolean_shape()
        } else if matches!(op, "<<" | ">>") {
            a.clone()
        } else {
            common.unwrap()
        }
    }

    pub(crate) fn require_writable(&mut self, expr: &Expr) {
        if !self.writable(expr) {
            self.error("T015", expr.span, "expression is not a writable location");
        }
    }

    fn writable(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Name(name) => self.lookup(name).is_some_and(|b| b.writable),
            ExprKind::Member { base, name } => {
                let unique =
                    if matches!(self.expression_types.get(&base.id), Some(Type::Vector(..))) {
                        swizzle(name)
                            .is_some_and(|v| v.iter().enumerate().all(|(i, x)| !v[..i].contains(x)))
                    } else {
                        true
                    };
                unique && self.writable(base)
            }
            ExprKind::Index { base, .. } => match self.expression_types.get(&base.id) {
                Some(Type::Buffer { writable, .. } | Type::Texture { writable, .. }) => *writable,
                _ => self.writable(base),
            },
            _ => false,
        }
    }
}

fn swizzle(name: &str) -> Option<Vec<u8>> {
    if name.is_empty() {
        return None;
    }
    for alphabet in ["xyzw", "rgba"] {
        if let Some(indices) = name
            .chars()
            .map(|ch| alphabet.find(ch).map(|i| i as u8))
            .collect::<Option<Vec<_>>>()
        {
            return Some(indices);
        }
    }
    None
}
