use crate::{Initialization, InitializationKind, Type, checker::Checker, span_key};
use slang_syntax::{Declaration, Expr, ExprKind, Module};
use std::collections::BTreeSet;

enum Work<'a> {
    Expression(&'a Expr),
    Initialization(&'a Initialization),
}

impl Checker {
    pub(crate) fn validate_static_initializers(&mut self, module: &Module) {
        let constructors: BTreeSet<_> = module
            .declarations
            .iter()
            .filter_map(|d| {
                if let Declaration::Function(f) = d
                    && f.constructor.is_some()
                {
                    Some(
                        self.function_names
                            .get(&span_key(f.span))
                            .unwrap_or(&f.name)
                            .clone(),
                    )
                } else {
                    None
                }
            })
            .collect();
        for declaration in &module.declarations {
            let Declaration::Global(variable) = declaration else {
                continue;
            };
            if !["static", "const"]
                .iter()
                .all(|m| variable.modifiers.iter().any(|v| v == m))
            {
                continue;
            }
            if self.static_declaration(&variable.name) {
                continue;
            }
            let Some(value) = &variable.initializer else {
                continue;
            };
            self.current_module = self
                .symbols
                .get(&variable.name)
                .map_or(slang_module::ModuleId(0), |o| o.module);
            if self.integer_constant(value).is_some() {
                continue;
            }
            if self.runtime_initializer(value, &constructors) {
                self.error(
                    "T031",
                    value.span,
                    "static const global initializer must be a compile-time constant",
                );
            }
        }
    }

    fn runtime_initializer(&self, root: &Expr, constructors: &BTreeSet<String>) -> bool {
        let mut work = vec![Work::Expression(root)];
        let mut seen = BTreeSet::new();
        while let Some(value) = work.pop() {
            match value {
                Work::Initialization(i) => match &i.kind {
                    InitializationKind::Value(e) | InitializationKind::DeclarationDefault(e) => {
                        work.push(Work::Expression(e))
                    }
                    InitializationKind::Aggregate(values) => {
                        work.extend(values.iter().map(Work::Initialization))
                    }
                    InitializationKind::ConstructorCall { arguments, .. } => {
                        work.extend(arguments.iter().map(Work::Initialization))
                    }
                    InitializationKind::Zero => {}
                },
                Work::Expression(e) => {
                    if !seen.insert(e.id) || self.expression_types.get(&e.id) == Some(&Type::Error)
                    {
                        continue;
                    }
                    if self.static_references.contains_key(&e.id) {
                        continue;
                    }
                    match &e.kind {
                        ExprKind::Name(name) => {
                            if let Some(global) = self.shape_globals.get(name)
                                && !self.static_declaration(name)
                                && !["static", "const"]
                                    .iter()
                                    .all(|m| global.modifiers.iter().any(|v| v == m))
                            {
                                return true;
                            }
                        }
                        ExprKind::Call { callee, arguments } => {
                            if let Some(name) = self.call_names.get(&callee.id)
                                && matches!(
                                    self.expression_types.get(&callee.id),
                                    Some(Type::Callable { .. })
                                )
                                && !constructors.contains(name)
                            {
                                return true;
                            }
                            work.extend(arguments.iter().map(Work::Expression));
                            if let Some(defaults) = self.default_arguments.get(&callee.id) {
                                work.extend(defaults.iter().map(Work::Initialization));
                            }
                            if let Some(plan) = self.initializations.get(&callee.id) {
                                work.push(Work::Initialization(plan));
                            }
                            if let ExprKind::Member { base, .. } = &callee.kind
                                && !matches!(
                                    self.method_receivers.get(&callee.id),
                                    Some(crate::MethodReceiver::Static)
                                )
                            {
                                work.push(Work::Expression(base));
                            }
                        }
                        ExprKind::InitializerList(values) => {
                            work.extend(values.iter().map(Work::Expression));
                            if let Some(plan) = self.initializations.get(&e.id) {
                                work.push(Work::Initialization(plan));
                            }
                        }
                        ExprKind::Unary { value, .. }
                        | ExprKind::Postfix { value, .. }
                        | ExprKind::Cast { value, .. } => work.push(Work::Expression(value)),
                        ExprKind::Binary { left, right, .. } => {
                            work.push(Work::Expression(left));
                            work.push(Work::Expression(right));
                        }
                        ExprKind::Member { base, .. } => work.push(Work::Expression(base)),
                        ExprKind::Index { base, index } => {
                            work.push(Work::Expression(base));
                            work.push(Work::Expression(index));
                        }
                        ExprKind::Conditional {
                            condition,
                            then_value,
                            else_value,
                        } => work.extend(
                            [condition, then_value, else_value]
                                .into_iter()
                                .map(|v| Work::Expression(v)),
                        ),
                        _ => {}
                    }
                }
            }
        }
        false
    }
}
