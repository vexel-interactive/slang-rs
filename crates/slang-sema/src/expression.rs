use crate::{Scalar, Type, checker::Checker};
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
                if !matches!(
                    index_type,
                    Type::Scalar(Scalar::Int | Scalar::Uint) | Type::Error
                ) {
                    self.error("T013", index.span, "index must be a scalar integer");
                }
                let (result, size) = match ty {
                    Type::Array(element, size) => (*element, Some(size)),
                    Type::Vector(s, n) => (Type::Scalar(s), Some(u32::from(n))),
                    Type::Buffer { element, .. } => (*element, None),
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

    fn call(&mut self, callee: &Expr, arguments: &[Expr], span: Span) -> Type {
        let types: Vec<Type> = arguments.iter().map(|a| self.expression(a)).collect();
        let ExprKind::Name(name) = &callee.kind else {
            self.error("T001", span, "only named function calls are implemented");
            return Type::Error;
        };
        if self.lookup(name).is_some() {
            self.error("T014", span, format!("value '{name}' is not callable"));
            return Type::Error;
        }
        let constructor = Type::builtin(name).or_else(|| {
            self.structures
                .contains_key(name)
                .then(|| Type::Struct(name.clone()))
        });
        if let Some(target) = constructor {
            let valid = match &target {
                Type::Scalar(_) => types.len() == 1 && matches!(types[0], Type::Scalar(_)),
                Type::Vector(scalar, width) => {
                    let components: Option<u16> =
                        types.iter().map(|t| t.width().map(u16::from)).sum();
                    let scalar_types_valid = types.iter().all(|t| {
                        t.scalar()
                            .is_some_and(|s| (s == Scalar::Bool) == (*scalar == Scalar::Bool))
                    });
                    scalar_types_valid
                        && (components == Some(u16::from(*width))
                            || types.len() == 1 && types[0].width() == Some(1))
                }
                Type::Struct(structure) => {
                    // Struct constructor field order is not implemented; reject explicitly.
                    self.error(
                        "T001",
                        span,
                        format!("constructor for struct '{structure}' is not implemented"),
                    );
                    false
                }
                _ => false,
            };
            if !valid {
                self.error(
                    "T014",
                    span,
                    format!("invalid constructor arguments for {target}"),
                );
            }
            self.expression_types
                .insert(callee.id, Type::Meta(Box::new(target.clone())));
            return target;
        }
        if let Some(overloads) = self.functions.get(name).cloned() {
            let mut matches: Vec<_> = overloads
                .into_iter()
                .filter(|s| {
                    s.parameters.len() == types.len()
                        && types
                            .iter()
                            .zip(&s.parameters)
                            .all(|(a, b)| a.convertible_to(b))
                })
                .collect();
            matches.sort_by_key(|s| {
                types
                    .iter()
                    .zip(&s.parameters)
                    .filter(|(a, b)| a != b)
                    .count()
            });
            let best = matches.first();
            if let Some(best) = best {
                let score = types
                    .iter()
                    .zip(&best.parameters)
                    .filter(|(a, b)| a != b)
                    .count();
                if matches.get(1).is_some_and(|s| {
                    types
                        .iter()
                        .zip(&s.parameters)
                        .filter(|(a, b)| a != b)
                        .count()
                        == score
                }) {
                    self.error("T014", span, format!("ambiguous call to '{name}'"));
                }
                if !best.defined {
                    self.error("T014", span, format!("function '{name}' has no definition"));
                }
                for (argument, output) in arguments.iter().zip(&best.output_parameters) {
                    if *output {
                        self.require_writable(argument);
                    }
                }
                self.expression_types.insert(
                    callee.id,
                    Type::Callable {
                        parameters: best.parameters.clone(),
                        result: Box::new(best.result.clone()),
                    },
                );
                return best.result.clone();
            }
            self.error("T014", span, format!("no matching overload for '{name}'"));
            return Type::Error;
        }
        if let Some(result) = intrinsic(name, &types) {
            self.expression_types.insert(
                callee.id,
                Type::Callable {
                    parameters: types,
                    result: Box::new(result.clone()),
                },
            );
            return result;
        }
        self.error(
            "T014",
            span,
            format!("unknown function or unsupported intrinsic '{name}'"),
        );
        Type::Error
    }

    fn require_writable(&mut self, expr: &Expr) {
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
                Some(Type::Buffer { writable, .. }) => *writable,
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

fn intrinsic(name: &str, types: &[Type]) -> Option<Type> {
    match (name, types) {
        ("abs", [a]) if a.numeric() => Some(a.clone()),
        (
            "sin" | "cos" | "tan" | "sqrt" | "rsqrt" | "floor" | "ceil" | "frac" | "exp" | "exp2"
            | "log" | "log2" | "saturate",
            [a],
        ) if matches!(
            a.scalar(),
            Some(Scalar::Half | Scalar::Float | Scalar::Double)
        ) =>
        {
            Some(a.clone())
        }
        ("min" | "max" | "pow", [a, b]) if a.numeric() && b.numeric() => Type::common(a, b),
        ("clamp" | "lerp", [a, b, c]) if a.numeric() && b.numeric() && c.numeric() => {
            Type::common(&Type::common(a, b)?, c)
        }
        ("dot", [a, b])
            if a == b
                && matches!(
                    a,
                    Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
                ) =>
        {
            Some(Type::Scalar(a.scalar()?))
        }
        ("length", [a])
            if matches!(
                a,
                Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
            ) =>
        {
            Some(Type::Scalar(a.scalar()?))
        }
        ("normalize", [a])
            if matches!(
                a,
                Type::Vector(Scalar::Half | Scalar::Float | Scalar::Double, _)
            ) =>
        {
            Some(a.clone())
        }
        ("all" | "any", [a]) if a.scalar().is_some() => Some(Type::Scalar(Scalar::Bool)),
        (
            "GroupMemoryBarrierWithGroupSync"
            | "DeviceMemoryBarrierWithGroupSync"
            | "AllMemoryBarrierWithGroupSync",
            [],
        ) => Some(Type::Void),
        _ => None,
    }
}
