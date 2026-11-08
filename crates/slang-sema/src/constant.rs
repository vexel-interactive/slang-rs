use crate::{Scalar, Type, checker::Checker};
use slang_syntax::{Declaration, Expr, ExprKind, Module};
use std::collections::{BTreeMap, BTreeSet};

impl Checker {
    /// Resolve checked integer globals by reverse dependency edges, including forwards.
    pub(crate) fn resolve_global_constants(&mut self, module: &Module) {
        let globals: BTreeMap<_, _> = module
            .declarations
            .iter()
            .filter_map(|declaration| {
                let Declaration::Global(variable) = declaration else {
                    return None;
                };
                (variable.modifiers.iter().any(|m| m == "const")
                    && variable.initializer.is_some()
                    && matches!(
                        self.declaration_types.get(&crate::span_key(variable.span)),
                        Some(Type::Scalar(
                            Scalar::Int16
                                | Scalar::Uint16
                                | Scalar::Int
                                | Scalar::Uint
                                | Scalar::Int64
                                | Scalar::Uint64
                                | Scalar::IntPtr
                                | Scalar::UintPtr
                                | Scalar::Bool
                        ))
                    ))
                .then_some((crate::span_key(variable.span), variable))
            })
            .collect();
        for variable in globals.values() {
            if let Some(binding) = self.scopes[0].get_mut(&variable.name) {
                binding.constant = None;
            }
        }
        let mut dependents: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut dependencies = BTreeMap::new();
        for (&key, variable) in &globals {
            let references = global_dependencies(
                variable.initializer.as_ref().unwrap(),
                &self.global_references,
            )
            .into_iter()
            .filter(|dependency| globals.contains_key(dependency))
            .collect::<BTreeSet<_>>();
            for &dependency in &references {
                dependents.entry(dependency).or_default().push(key);
            }
            dependencies.insert(key, references);
        }
        for key in constant_cycles(&dependencies) {
            let variable = globals[&key];
            self.error(
                "T026",
                variable.initializer.as_ref().unwrap().span,
                format!(
                    "initializer for integer constant '{}' depends on itself",
                    self.source_name(&variable.name)
                ),
            );
        }
        let previous_module = self.current_module;
        let mut pending: Vec<_> = globals.keys().copied().collect();
        while let Some(key) = pending.pop() {
            let variable = globals[&key];
            if self.scopes[0]
                .get(&variable.name)
                .is_some_and(|b| b.constant.is_some())
            {
                continue;
            }
            self.current_module = self
                .symbols
                .get(&variable.name)
                .map_or(slang_module::ModuleId(0), |origin| origin.module);
            if let Some(value) = self.integer_constant(variable.initializer.as_ref().unwrap()) {
                if let Some(binding) = self.scopes[0].get_mut(&variable.name) {
                    binding.constant = Some(normalize(value, &binding.ty));
                }
                pending.extend(dependents.get(&key).into_iter().flatten().copied());
            }
        }
        self.current_module = previous_module;
    }

    /// Fold typed 32/64-bit integer constants, storing unsigned values as bits.
    pub(crate) fn integer_constant(&self, expr: &Expr) -> Option<i64> {
        self.integer_constant_mode(expr, false)
    }

    fn integer_constant_mode(&self, expr: &Expr, shape: bool) -> Option<i64> {
        if let Ok(Some(literal)) = slang_syntax::integer_literal(expr) {
            let value = literal.bits as i64;
            return Some(if shape {
                value
            } else {
                self.expression_types
                    .get(&expr.id)
                    .map_or(value, |ty| normalize(value, ty))
            });
        }
        if matches!(
            self.expression_types.get(&expr.id),
            Some(Type::Scalar(Scalar::Half | Scalar::Float | Scalar::Double))
        ) {
            return None;
        }
        let value = match &expr.kind {
            ExprKind::Integer(_) => return None,
            ExprKind::InitializerList(values) if values.len() == 1 => {
                self.integer_constant_mode(&values[0], shape)?
            }
            ExprKind::Bool(value) => i64::from(*value),
            ExprKind::Name(name) => {
                let binding = self.lookup(name)?;
                if shape {
                    binding.shape_constant.or(binding.constant)?
                } else {
                    binding.constant?
                }
            }
            ExprKind::Unary { op, value } => {
                let value = if op == "!" {
                    self.constant_conversion_mode(value, shape)?
                } else {
                    self.integer_constant_mode(value, shape)?
                };
                match op.as_str() {
                    "+" => value,
                    "-" => value.wrapping_neg(),
                    "!" => i64::from(value == 0),
                    "~" => !value,
                    _ => return None,
                }
            }
            ExprKind::Binary { op, left, right } => {
                let mut a = self.integer_constant_mode(left, shape)?;
                if op == "&&" && a == 0 {
                    return Some(0);
                }
                if op == "||" && a != 0 {
                    return Some(1);
                }
                let mut b = self.integer_constant_mode(right, shape)?;
                let common = self
                    .expression_types
                    .get(&left.id)
                    .zip(self.expression_types.get(&right.id))
                    .and_then(|(a, b)| Type::common(a, b));
                if !shape
                    && !matches!(op.as_str(), "<<" | ">>")
                    && let Some(common) = &common
                {
                    a = normalize(a, common);
                    b = normalize(b, common);
                }
                let operation_type = if matches!(op.as_str(), "<<" | ">>") {
                    self.expression_types.get(&left.id)
                } else {
                    common.as_ref()
                };
                let unsigned64 = operation_type.is_some_and(|ty| {
                    matches!(ty.scalar(), Some(Scalar::Uint64 | Scalar::UintPtr))
                });
                if !shape
                    && matches!(op.as_str(), "<<" | ">>")
                    && operation_type.is_some_and(|ty| {
                        matches!(ty.scalar(), Some(Scalar::Int16 | Scalar::Uint16))
                    })
                    && !(0..16).contains(&b)
                {
                    return Some(i64::from(op == ">>" && a < 0).wrapping_neg());
                }
                let shift_bits = if shape
                    || operation_type.is_some_and(|ty| {
                        matches!(
                            ty.scalar(),
                            Some(Scalar::Int64 | Scalar::Uint64 | Scalar::IntPtr | Scalar::UintPtr)
                        )
                    }) {
                    64
                } else {
                    32
                };
                match op.as_str() {
                    "+" => a.wrapping_add(b),
                    "-" => a.wrapping_sub(b),
                    "*" => a.wrapping_mul(b),
                    "/" if unsigned64 => (a as u64).checked_div(b as u64)? as i64,
                    "%" if unsigned64 => (a as u64).checked_rem(b as u64)? as i64,
                    "/" => a.checked_div(b)?,
                    "%" => a.checked_rem(b)?,
                    "&" => a & b,
                    "|" => a | b,
                    "^" => a ^ b,
                    "<<" if (0..shift_bits).contains(&b) => a << b,
                    ">>" if (0..shift_bits).contains(&b) && unsigned64 => ((a as u64) >> b) as i64,
                    ">>" if (0..shift_bits).contains(&b) => a >> b,
                    "==" => i64::from(a == b),
                    "!=" => i64::from(a != b),
                    "<" if unsigned64 => i64::from((a as u64) < b as u64),
                    "<=" if unsigned64 => i64::from((a as u64) <= b as u64),
                    ">" if unsigned64 => i64::from((a as u64) > b as u64),
                    ">=" if unsigned64 => i64::from((a as u64) >= b as u64),
                    "<" => i64::from(a < b),
                    "<=" => i64::from(a <= b),
                    ">" => i64::from(a > b),
                    ">=" => i64::from(a >= b),
                    "&&" => i64::from(a != 0 && b != 0),
                    "||" => i64::from(a != 0 || b != 0),
                    _ => return None,
                }
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => self.integer_constant_mode(
                if self.integer_constant_mode(condition, shape)? != 0 {
                    then_value
                } else {
                    else_value
                },
                shape,
            )?,
            ExprKind::Cast { value, .. } => {
                self.convert_constant_to(value, self.expression_types.get(&expr.id)?, shape)?
            }
            ExprKind::Call { callee, arguments }
                if arguments.len() == 1
                    && matches!(self.expression_types.get(&callee.id), Some(Type::Meta(target)) if matches!(target.as_ref(), Type::Scalar(Scalar::Int16 | Scalar::Uint16 | Scalar::Int | Scalar::Uint | Scalar::Int64 | Scalar::Uint64 | Scalar::IntPtr | Scalar::UintPtr | Scalar::Bool))) =>
            {
                self.convert_constant_to(
                    &arguments[0],
                    self.expression_types.get(&expr.id)?,
                    shape,
                )?
            }
            _ => return None,
        };
        Some(
            if shape && self.expression_types.get(&expr.id) != Some(&Type::Scalar(Scalar::Bool)) {
                value
            } else {
                self.expression_types
                    .get(&expr.id)
                    .map_or(value, |ty| normalize(value, ty))
            },
        )
    }

    pub(crate) fn constant_conversion(&self, expr: &Expr) -> Option<i64> {
        self.constant_conversion_mode(expr, false)
    }

    /// Type-level folding preserves 64-bit intermediates before shape checking.
    pub(crate) fn shape_constant_conversion(&self, expr: &Expr) -> Option<i64> {
        self.constant_conversion_mode(expr, true)
    }

    /// Shape arithmetic keeps wide intermediates, but a conversion between
    /// different scalar types truncates at the target width. Identity
    /// constructors retain the intermediate, as in TypeCastIntVal folding.
    pub(crate) fn shape_initializer_conversion(&self, expr: &Expr, target: &Type) -> Option<i64> {
        self.convert_constant_to(expr, target, true)
    }

    fn convert_constant_to(&self, expr: &Expr, target: &Type, shape: bool) -> Option<i64> {
        let value = self.constant_conversion_mode(expr, shape)?;
        Some(
            if !shape || self.expression_types.get(&expr.id) != Some(target) {
                normalize(value, target)
            } else {
                value
            },
        )
    }

    fn constant_conversion_mode(&self, expr: &Expr, shape: bool) -> Option<i64> {
        if let Some(value) = self.floating_literal(expr) {
            (value.is_finite() && value >= i64::MIN as f64 && value < 2.0_f64.powi(64)).then_some(
                if value >= 2.0_f64.powi(63) {
                    (value as u64) as i64
                } else {
                    value as i64
                },
            )
        } else {
            self.integer_constant_mode(expr, shape)
        }
    }

    /// Prefix signs fold literals in the upstream parser. A conversion to the
    /// same type is also removed before integer constant evaluation.
    fn floating_literal(&self, expr: &Expr) -> Option<f64> {
        if expr.parenthesized {
            return None;
        }
        match &expr.kind {
            ExprKind::Float(_) | ExprKind::Unary { .. } => parser_floating_literal(expr),
            ExprKind::Cast { value, .. }
                if self.expression_types.get(&expr.id) == self.expression_types.get(&value.id) =>
            {
                self.floating_literal(value)
            }
            ExprKind::Call { callee, arguments }
                if arguments.len() == 1
                    && matches!(self.expression_types.get(&callee.id), Some(Type::Meta(target))
                    if Some(target.as_ref()) == self.expression_types.get(&arguments[0].id)) =>
            {
                self.floating_literal(&arguments[0])
            }
            _ => None,
        }
    }
}

/// Only syntactic literals fold through prefix signs. Identity constructors
/// are simplified later, so a sign outside such a constructor is not a literal.
fn parser_floating_literal(expr: &Expr) -> Option<f64> {
    if expr.parenthesized {
        return None;
    }
    match &expr.kind {
        ExprKind::Float(text) => slang_lexer::parse_float_literal(text).ok().map(|v| v.value),
        ExprKind::Unary { op, value } if matches!(op.as_str(), "+" | "-") => {
            let value = parser_floating_literal(value)?;
            Some(if op == "-" { -value } else { value })
        }
        _ => None,
    }
}

fn constant_cycles(
    dependencies: &BTreeMap<crate::checker::SpanKey, BTreeSet<crate::checker::SpanKey>>,
) -> BTreeSet<crate::checker::SpanKey> {
    let mut states = BTreeMap::new();
    let mut cycles = BTreeSet::new();
    for &root in dependencies.keys() {
        let mut pending = vec![(root, false)];
        while let Some((key, exiting)) = pending.pop() {
            if exiting {
                states.insert(key, 2);
                continue;
            }
            if states.get(&key).is_some_and(|state| *state != 0) {
                continue;
            }
            states.insert(key, 1);
            pending.push((key, true));
            for &dependency in dependencies[&key].iter().rev() {
                match states.get(&dependency).copied().unwrap_or(0) {
                    1 => {
                        cycles.insert(key);
                    }
                    0 => pending.push((dependency, false)),
                    _ => {}
                }
            }
        }
    }
    cycles
}

fn global_dependencies(
    expression: &Expr,
    references: &BTreeMap<slang_syntax::ExprId, crate::checker::SpanKey>,
) -> BTreeSet<crate::checker::SpanKey> {
    let mut pending = vec![expression];
    let mut dependencies = BTreeSet::new();
    while let Some(expression) = pending.pop() {
        if let Some(&reference) = references.get(&expression.id) {
            dependencies.insert(reference);
        }
        match &expression.kind {
            ExprKind::InitializerList(values) => pending.extend(values),
            ExprKind::Unary { value, .. }
            | ExprKind::Postfix { value, .. }
            | ExprKind::Cast { value, .. } => pending.push(value),
            ExprKind::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprKind::Call { callee, arguments } => {
                pending.push(callee);
                pending.extend(arguments);
            }
            ExprKind::Member { base, .. } => pending.push(base),
            ExprKind::Index { base, index } => pending.extend([base.as_ref(), index.as_ref()]),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => pending.extend([condition.as_ref(), then_value.as_ref(), else_value.as_ref()]),
            _ => {}
        }
    }
    dependencies
}

pub(crate) fn normalize(value: i64, ty: &Type) -> i64 {
    match ty {
        Type::Scalar(Scalar::Int16) => i64::from(value as i16),
        Type::Scalar(Scalar::Uint16) => i64::from(value as u16),
        Type::Scalar(Scalar::Int) => i64::from(value as i32),
        Type::Scalar(Scalar::Uint) => i64::from(value as u32),
        Type::Scalar(Scalar::Bool) => i64::from(value != 0),
        _ => value,
    }
}
