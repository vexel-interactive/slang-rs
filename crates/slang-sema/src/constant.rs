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
                        Some(Type::Scalar(Scalar::Int | Scalar::Uint | Scalar::Bool))
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

    /// Fold the implemented scalar integer constant subset using 32-bit shapes.
    pub(crate) fn integer_constant(&self, expr: &Expr) -> Option<i64> {
        if matches!(
            self.expression_types.get(&expr.id),
            Some(Type::Scalar(Scalar::Half | Scalar::Float | Scalar::Double))
        ) {
            return None;
        }
        let value = match &expr.kind {
            ExprKind::Integer(text) => integer(text)?,
            ExprKind::InitializerList(values) if values.len() == 1 => {
                self.integer_constant(&values[0])?
            }
            ExprKind::Bool(value) => i64::from(*value),
            ExprKind::Name(name) => self.lookup(name)?.constant?,
            ExprKind::Unary { op, value } => {
                let value = self.integer_constant(value)?;
                match op.as_str() {
                    "+" => value,
                    "-" => value.wrapping_neg(),
                    "!" => i64::from(value == 0),
                    "~" => !value,
                    _ => return None,
                }
            }
            ExprKind::Binary { op, left, right } => {
                let mut a = self.integer_constant(left)?;
                if op == "&&" && a == 0 {
                    return Some(0);
                }
                if op == "||" && a != 0 {
                    return Some(1);
                }
                let mut b = self.integer_constant(right)?;
                if !matches!(op.as_str(), "<<" | ">>")
                    && let Some(common) = self
                        .expression_types
                        .get(&left.id)
                        .zip(self.expression_types.get(&right.id))
                        .and_then(|(a, b)| Type::common(a, b))
                {
                    a = normalize(a, &common);
                    b = normalize(b, &common);
                }
                match op.as_str() {
                    "+" => a.wrapping_add(b),
                    "-" => a.wrapping_sub(b),
                    "*" => a.wrapping_mul(b),
                    "/" => a.checked_div(b)?,
                    "%" => a.checked_rem(b)?,
                    "&" => a & b,
                    "|" => a | b,
                    "^" => a ^ b,
                    "<<" if (0..32).contains(&b) => a << b,
                    ">>" if (0..32).contains(&b) => a >> b,
                    "==" => i64::from(a == b),
                    "!=" => i64::from(a != b),
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
            } => self.integer_constant(if self.integer_constant(condition)? != 0 {
                then_value
            } else {
                else_value
            })?,
            ExprKind::Cast { value, .. } => self.constant_conversion(value)?,
            ExprKind::Call { callee, arguments }
                if arguments.len() == 1
                    && matches!(self.expression_types.get(&callee.id), Some(Type::Meta(target)) if matches!(target.as_ref(), Type::Scalar(Scalar::Int | Scalar::Uint | Scalar::Bool))) =>
            {
                self.constant_conversion(&arguments[0])?
            }
            _ => return None,
        };
        Some(
            self.expression_types
                .get(&expr.id)
                .map_or(value, |ty| normalize(value, ty)),
        )
    }

    pub(crate) fn constant_conversion(&self, expr: &Expr) -> Option<i64> {
        if let ExprKind::Float(text) = &expr.kind {
            let value = floating(text)?;
            (value.is_finite() && value >= i32::MIN as f64 && value <= u32::MAX as f64)
                .then_some(value as i64)
        } else {
            self.integer_constant(expr)
        }
    }
}

fn floating(text: &str) -> Option<f64> {
    let value = text
        .trim_end_matches(['f', 'F', 'h', 'H'])
        .parse::<f64>()
        .ok()?;
    value.is_finite().then_some(value)
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
        Type::Scalar(Scalar::Int) => i64::from(value as i32),
        Type::Scalar(Scalar::Uint) => i64::from(value as u32),
        Type::Scalar(Scalar::Bool) => i64::from(value != 0),
        _ => value,
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
