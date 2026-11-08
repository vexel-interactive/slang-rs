use crate::{
    Scalar, TextureDimension, Type,
    checker::{Checker, Storage},
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Expr, ExprKind};

impl Checker {
    pub(crate) fn value_expression(&mut self, expression: &Expr) -> Type {
        let ty = self.expression(expression);
        self.load_type(expression, ty)
    }

    fn load_type(&mut self, expression: &Expr, ty: Type) -> Type {
        if let Type::ConstantBuffer { element, .. } = ty {
            self.constant_buffer_loads.insert(expression.id);
            *element
        } else {
            ty
        }
    }

    pub fn expression(&mut self, expression: &Expr) -> Type {
        let ty = match &expression.kind {
            ExprKind::InitializerList(values) => {
                for value in values {
                    self.expression(value);
                }
                Type::InitializerList
            }
            ExprKind::Integer(text) => {
                let digits = text.trim_end_matches(['u', 'U', 'l', 'L']);
                let value = slang_lexer::integer_literal_value(text);
                if text.to_ascii_lowercase().contains("ll") {
                    self.error(
                        "T001",
                        expression.span,
                        "64-bit integer literals are not implemented",
                    );
                }
                let unsigned =
                    text.contains(['u', 'U']) || value.is_some_and(|v| v > i32::MAX as u64);
                if digits.starts_with('-') {
                    self.error(
                        "T010",
                        expression.span,
                        "negative __LINE__ values cannot form integer literals",
                    );
                } else if value.is_none_or(|v| v > u32::MAX as u64) {
                    self.error("T010", expression.span, "integer literal exceeds 32 bits");
                }
                Type::Scalar(if unsigned { Scalar::Uint } else { Scalar::Int })
            }
            ExprKind::Float(text) => match slang_lexer::parse_float_literal(text) {
                Ok(literal) => {
                    let scalar = match literal.kind {
                        slang_lexer::FloatKind::Half => Scalar::Half,
                        slang_lexer::FloatKind::Float => Scalar::Float,
                        slang_lexer::FloatKind::Double => Scalar::Double,
                    };
                    let warning = if literal.out_of_range {
                        Some(if literal.value.is_finite() {
                            ("T027", "floating-point literal underflows to zero")
                        } else {
                            ("T028", "floating-point literal overflows to infinity")
                        })
                    } else if literal.precision_lost {
                        Some(("T029", "hexadecimal floating-point literal loses precision"))
                    } else {
                        None
                    };
                    if let Some((code, message)) = warning
                        && !self
                            .diagnostics
                            .iter()
                            .any(|d| d.code == code && d.span == expression.span)
                    {
                        self.diagnostics
                            .push(Diagnostic::warning(code, expression.span, message));
                    }
                    Type::Scalar(scalar)
                }
                Err(_) => {
                    self.error("T010", expression.span, "invalid floating-point literal");
                    Type::Error
                }
            },
            ExprKind::Bool(_) => Type::Scalar(Scalar::Bool),
            ExprKind::Cast { target, value } => {
                let actual = self.value_expression(value);
                let target = self.resolve_type(target);
                let valid = actual == Type::Error
                    || target == Type::Error
                    || match (&actual, &target) {
                        (
                            Type::Scalar(_),
                            Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..),
                        ) => true,
                        (Type::Vector(_, a), Type::Vector(_, b)) => a == b,
                        (Type::Vector(_, 1), Type::Scalar(_)) => true,
                        (Type::Matrix(_, ar, ac), Type::Matrix(_, br, bc)) => br <= ar && bc <= ac,
                        _ => false,
                    };
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
                if let Some(binding) = self.lookup(name).cloned() {
                    if let Storage::Global(declaration) | Storage::GroupShared(declaration) =
                        binding.storage
                    {
                        self.global_references.insert(expression.id, declaration);
                    }
                    binding.ty
                } else {
                    self.error("T011", expression.span, format!("unknown value '{name}'"));
                    Type::Error
                }
            }
            ExprKind::TypeApplication(_) => {
                self.error(
                    "T001",
                    expression.span,
                    "type applications require a constructor call",
                );
                Type::Error
            }
            ExprKind::Unary { op, value } => {
                let ty = self.value_expression(value);
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
                let ty = self.value_expression(value);
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
                if op == "=" {
                    self.require_writable(left);
                    self.check_value(right, &a);
                    a
                } else if matches!(
                    op.as_str(),
                    "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                ) {
                    let b = self.value_expression(right);
                    self.require_writable(left);
                    let result = self.binary(op.trim_end_matches('='), &a, &b, expression.span);
                    self.require_conversion(&result, &a, expression.span);
                    a
                } else {
                    let a = self.load_type(left, a);
                    let b = if op == "," {
                        self.expression(right)
                    } else {
                        self.value_expression(right)
                    };
                    self.binary(op, &a, &b, expression.span)
                }
            }
            ExprKind::Call { callee, arguments } => self.call(callee, arguments, expression.span),
            ExprKind::Member { base, name } => {
                let ty = self.value_expression(base);
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
                            crate::swizzle::vector(name).filter(|v| v.iter().all(|&i| i < width))
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
                    Type::Matrix(scalar, rows, columns) => {
                        if let Some(indices) = crate::swizzle::matrix(name, rows, columns) {
                            Type::numeric_shape(scalar, indices.len() as u8)
                        } else {
                            self.error(
                                "T013",
                                expression.span,
                                format!("invalid matrix swizzle '{name}'"),
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
                let ty = self.value_expression(base);
                let index_type = self.value_expression(index);
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
                    && self
                        .integer_constant(index)
                        .is_some_and(|v| v < 0 || v >= i64::from(size))
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
                let mut a = self.expression(then_value);
                let mut b = self.expression(else_value);
                if a != b {
                    a = self.load_type(then_value, a);
                    b = self.load_type(else_value, b);
                }
                if a == Type::InitializerList && b == Type::InitializerList {
                    self.error(
                        "T021",
                        expression.span,
                        "conditional branches require a concrete type",
                    );
                    return Type::Error;
                }
                if a == Type::InitializerList && b != Type::InitializerList {
                    self.coerce_value(then_value, &b);
                    a = b.clone();
                } else if b == Type::InitializerList && a != Type::InitializerList {
                    self.coerce_value(else_value, &a);
                    b = a.clone();
                }
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
        if op == "," {
            return b.clone();
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
        if self.constant_buffer_loads.contains(&expr.id) {
            return false;
        }
        match &expr.kind {
            ExprKind::Name(name) => self.lookup(name).is_some_and(|b| b.writable),
            ExprKind::Member { base, name } => {
                if matches!(
                    self.expression_types.get(&base.id),
                    Some(Type::ConstantBuffer { .. })
                ) {
                    return false;
                }
                let unique = match self.expression_types.get(&base.id) {
                    Some(Type::Vector(..)) => {
                        crate::swizzle::vector(name).is_some_and(|v| crate::swizzle::unique(&v))
                    }
                    Some(Type::Matrix(_, rows, columns)) => {
                        crate::swizzle::matrix(name, *rows, *columns)
                            .is_some_and(|v| crate::swizzle::unique(&v))
                    }
                    _ => true,
                };
                unique && self.writable(base)
            }
            ExprKind::Index { base, .. } => match self.expression_types.get(&base.id) {
                Some(Type::ConstantBuffer { .. }) => false,
                Some(Type::Buffer { writable, .. } | Type::Texture { writable, .. }) => *writable,
                _ => self.writable(base),
            },
            _ => false,
        }
    }
}
