use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

/// Local static storage persists across calls within the shader invocation.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_static_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        globals: vec![],
        scopes: vec![],
    };
    for function in &mut module.functions {
        lowerer.scopes.push(
            function
                .parameters
                .iter()
                .map(|p| (p.name.clone(), None))
                .collect(),
        );
        if let Some(body) = &mut function.body {
            lowerer.statement(body)?;
        }
        lowerer.scopes.pop();
    }
    module.globals.extend(lowerer.globals);
    Ok(())
}

struct Lowerer {
    prefix: String,
    globals: Vec<Variable>,
    scopes: Vec<BTreeMap<String, Option<String>>>,
}

impl Lowerer {
    fn variable(&mut self, variable: &mut Variable) -> Result<Option<Stmt>, Diagnostic> {
        if let Some(value) = &mut variable.initializer {
            self.expression(value)?;
        }
        if !variable.modifiers.iter().any(|m| m == "static") {
            self.scopes
                .last_mut()
                .unwrap()
                .insert(variable.name.clone(), None);
            return Ok(None);
        }
        let name = format!("{}value_{}", self.prefix, self.globals.len());
        let mut storage = variable.clone();
        storage.name = name.clone();
        storage.modifiers = vec!["static".into()];
        storage.initializer = None;
        self.globals.push(storage);
        self.scopes
            .last_mut()
            .unwrap()
            .insert(variable.name.clone(), Some(name.clone()));
        let Some(initializer) = variable.initializer.take() else {
            return Ok(Some(Stmt {
                kind: StmtKind::Empty,
                span: variable.span,
            }));
        };
        let guard = format!("{}initialized_{}", self.prefix, self.globals.len());
        self.globals.push(Variable {
            name: guard.clone(),
            ty: Type::Scalar(Scalar::Bool),
            modifiers: vec!["static".into()],
            semantic: None,
            binding: None,
            uniform_layout: None,
            initializer: Some(Expr {
                kind: ExprKind::Bool(false),
                ty: Type::Scalar(Scalar::Bool),
                span: variable.span,
            }),
            span: variable.span,
        });
        let guard_value = binding(&guard, &Type::Scalar(Scalar::Bool), variable.span);
        Ok(Some(Stmt {
            span: variable.span,
            kind: StmtKind::If {
                condition: Expr {
                    ty: guard_value.ty.clone(),
                    span: variable.span,
                    kind: ExprKind::Unary {
                        op: "!".into(),
                        value: Box::new(guard_value.clone()),
                    },
                },
                then_branch: Box::new(Stmt {
                    span: variable.span,
                    kind: StmtKind::Block(vec![
                        assignment(binding(&name, &variable.ty, variable.span), initializer),
                        assignment(
                            guard_value,
                            Expr {
                                ty: Type::Scalar(Scalar::Bool),
                                kind: ExprKind::Bool(true),
                                span: variable.span,
                            },
                        ),
                    ]),
                }),
                else_branch: None,
            },
        }))
    }
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::Name(name) = &mut expression.kind {
            if let Some(Some(target)) = self.scopes.iter().rev().find_map(|scope| scope.get(name)) {
                *name = target.clone();
            }
        } else {
            walk_expression(self, expression)?;
        }
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Variable(variable) => {
                if let Some(replacement) = self.variable(variable)? {
                    *statement = replacement;
                }
            }
            StmtKind::Variables(variables) => {
                let mut statements = vec![];
                let mut changed = false;
                for variable in variables {
                    if let Some(replacement) = self.variable(variable)? {
                        changed = true;
                        statements.push(replacement);
                    } else {
                        statements.push(Stmt {
                            span: variable.span,
                            kind: StmtKind::Variable(variable.clone()),
                        });
                    }
                }
                if changed {
                    statement.kind = StmtKind::Sequence(statements);
                }
            }
            StmtKind::For { .. } => {
                self.scopes.push(BTreeMap::new());
                walk_statement(self, statement)?;
                self.scopes.pop();
                let StmtKind::For { initializer, .. } = &mut statement.kind else {
                    unreachable!()
                };
                if matches!(initializer.kind, StmtKind::If { .. }) {
                    **initializer = Stmt {
                        span: initializer.span,
                        kind: StmtKind::Sequence(vec![(**initializer).clone()]),
                    };
                }
            }
            StmtKind::Block(_) | StmtKind::Switch { .. } => {
                self.scopes.push(BTreeMap::new());
                walk_statement(self, statement)?;
                self.scopes.pop();
            }
            _ => walk_statement(self, statement)?,
        }
        Ok(())
    }
}

fn binding(name: &str, ty: &Type, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Name(name.into()),
        ty: ty.clone(),
        span,
    }
}

fn assignment(left: Expr, right: Expr) -> Stmt {
    Stmt {
        span: left.span,
        kind: StmtKind::Expression(Expr {
            ty: left.ty.clone(),
            span: left.span,
            kind: ExprKind::Binary {
                op: "=".into(),
                left: Box::new(left),
                right: Box::new(right),
            },
        }),
    }
}
