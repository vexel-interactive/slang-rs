use crate::{
    Scalar, Type,
    checker::{Binding, Checker, Storage, span_key},
};
use slang_syntax::{Expr, ExprKind};
use std::collections::BTreeSet;

impl Checker {
    /// Declaration types can precede global initialization. Fold their shape
    /// constants in an isolated global environment, keeping caller locals intact.
    pub(crate) fn numeric_shape_value(&mut self, expression: &Expr) -> Option<i64> {
        self.prepare_shape_names(expression);
        let saved = self.install_shape_bindings(expression);
        let ty = self.expression(expression);
        let value = if matches!(ty, Type::Scalar(_)) {
            self.shape_constant_conversion(expression)
        } else {
            None
        };
        self.restore_shape_bindings(saved);
        value
    }

    fn install_shape_bindings(&mut self, expression: &Expr) -> Vec<(String, Option<Binding>)> {
        let mut saved = vec![];
        for name in names(expression) {
            let Some(binding) = self.shape_bindings.get(&name) else {
                continue;
            };
            saved.push((name.clone(), self.scopes[0].insert(name, binding.clone())));
        }
        saved
    }

    fn restore_shape_bindings(&mut self, saved: Vec<(String, Option<Binding>)>) {
        for (name, binding) in saved {
            if let Some(binding) = binding {
                self.scopes[0].insert(name, binding);
            } else {
                self.scopes[0].remove(&name);
            }
        }
    }

    fn prepare_shape_names(&mut self, expression: &Expr) {
        for name in names(expression) {
            if self
                .lookup(&name)
                .is_some_and(|b| matches!(b.storage, Storage::Local))
            {
                continue;
            }
            self.prepare_shape_global(&name, expression.span);
        }
    }

    fn prepare_shape_global(&mut self, name: &str, span: slang_source::Span) {
        if self.shape_bindings.contains_key(name) || !self.module_symbol_visible(name) {
            return;
        }
        let Some(variable) = self.shape_globals.get(name).cloned() else {
            return;
        };
        if self.resolving_shape_globals.len() >= 32
            || !self.resolving_shape_globals.insert(name.into())
        {
            self.error(
                "T026",
                span,
                "cyclic or excessively nested numeric shape constant",
            );
            return;
        }
        let previous_module = self.current_module;
        self.current_module = self
            .symbols
            .get(name)
            .map_or(slang_module::ModuleId(0), |o| o.module);
        let mut scopes = std::mem::take(&mut self.scopes);
        self.scopes = vec![std::mem::take(&mut scopes[0])];
        let ty = self.resolve_type(&variable.ty);
        let constant = if variable.modifiers.iter().any(|m| m == "const")
            && matches!(
                ty,
                Type::Scalar(
                    Scalar::Int | Scalar::Uint | Scalar::Int64 | Scalar::Uint64 | Scalar::Bool
                )
            ) {
            variable.initializer.as_ref().and_then(|initializer| {
                self.prepare_shape_names(initializer);
                let saved = self.install_shape_bindings(initializer);
                self.expression(initializer);
                let value = self.shape_constant_conversion(initializer).map(|v| {
                    if ty == Type::Scalar(Scalar::Bool) {
                        i64::from(v != 0)
                    } else {
                        v
                    }
                });
                self.restore_shape_bindings(saved);
                value
            })
        } else {
            None
        };
        scopes[0] = std::mem::take(&mut self.scopes[0]);
        self.scopes = scopes;
        self.current_module = previous_module;
        self.resolving_shape_globals.remove(name);
        self.shape_bindings.insert(
            name.into(),
            Binding {
                ty,
                constant,
                shape_constant: constant,
                writable: false,
                storage: Storage::Global(span_key(variable.span)),
            },
        );
    }
}

fn names(expression: &Expr) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Name(name) => {
                names.insert(name.clone());
            }
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
            } => {
                pending.extend([condition.as_ref(), then_value.as_ref(), else_value.as_ref()]);
            }
            _ => {}
        }
    }
    names
}
