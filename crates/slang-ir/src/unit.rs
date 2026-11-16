//! Erase inferred unit storage while preserving the evaluation of its values.

use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::Diagnostic;

pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let mut lowerer = Lowerer;
    lowerer.module(module)?;
    for structure in &mut module.structures {
        structure.fields.retain(|field| field.ty != Type::Void);
    }
    for buffer in &mut module.constant_buffers {
        buffer.fields.retain(|field| field.ty != Type::Void);
        buffer.views.retain(|field| field.ty != Type::Void);
    }

    let mut startup = vec![];
    module.globals.retain_mut(|global| {
        if global.ty != Type::Void {
            return true;
        }
        if let Some(value) = global.initializer.take() {
            startup.push(Stmt {
                span: value.span,
                kind: StmtKind::Expression(value),
            });
        }
        false
    });
    if startup.is_empty() {
        return Ok(());
    }

    // Global unit declarations have no data, but their initializers execute
    // once at shader startup, before the entry point's source statements.
    let names = crate::visit::names(module);
    let mut prefix = "_slang_unit_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let span = startup[0].span;
    let initialized = format!("{prefix}initialized");
    let initialize = format!("{prefix}initialize");
    let guard = Expr {
        ty: Type::Scalar(Scalar::Bool),
        kind: ExprKind::GlobalName(initialized.clone()),
        span,
    };
    startup.insert(
        0,
        Stmt {
            span,
            kind: StmtKind::Expression(Expr {
                ty: guard.ty.clone(),
                kind: ExprKind::Binary {
                    op: "=".into(),
                    left: Box::new(guard.clone()),
                    right: Box::new(Expr {
                        ty: guard.ty.clone(),
                        kind: ExprKind::Bool(true),
                        span,
                    }),
                },
                span,
            }),
        },
    );
    module.globals.push(Variable {
        name: initialized,
        ty: guard.ty.clone(),
        modifiers: vec!["static".into()],
        semantic: None,
        binding: None,
        uniform_layout: None,
        pack_offset: None,
        initializer: Some(Expr {
            ty: guard.ty.clone(),
            kind: ExprKind::Bool(false),
            span,
        }),
        span,
    });
    module.functions.push(Function {
        name: initialize.clone(),
        return_type: Type::Void,
        parameters: vec![],
        modifiers: vec![],
        semantic: None,
        body: Some(Stmt {
            span,
            kind: StmtKind::Block(vec![Stmt {
                span,
                kind: StmtKind::If {
                    condition: Expr {
                        ty: guard.ty.clone(),
                        kind: ExprKind::Unary {
                            op: "!".into(),
                            value: Box::new(guard),
                        },
                        span,
                    },
                    then_branch: Box::new(Stmt {
                        span,
                        kind: StmtKind::Block(startup),
                    }),
                    else_branch: None,
                },
            }]),
        }),
        span,
    });
    for function in &mut module.functions {
        if module.entry_points.iter().any(|entry| entry.name == function.name)
            && let Some(body) = &mut function.body
        {
            let original = body.clone();
            body.kind = StmtKind::Block(vec![
                Stmt {
                    span,
                    kind: StmtKind::Expression(Expr {
                        ty: Type::Void,
                        kind: ExprKind::Call {
                            name: initialize.clone(),
                            arguments: vec![],
                        },
                        span,
                    }),
                },
                original,
            ]);
        }
    }
    Ok(())
}

struct Lowerer;

impl Visitor for Lowerer {
    fn expression(&mut self, value: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, value)?;
        if value.ty != Type::Void {
            return Ok(());
        }
        let discarded = match &value.kind {
            ExprKind::Name(_) | ExprKind::GlobalName(_) => vec![],
            ExprKind::Member {
                base,
                ..
            } => vec![*base.clone()],
            ExprKind::Index {
                base,
                index,
            } => vec![*base.clone(), *index.clone()],
            ExprKind::Binary {
                op,
                left,
                right,
            } if op == "=" => vec![*left.clone(), *right.clone()],
            ExprKind::Cast {
                value,
            } => vec![*value.clone()],
            _ => return Ok(()),
        };
        value.kind = if discarded.is_empty() {
            ExprKind::Zero
        } else {
            ExprKind::BlockExpression {
                statements: discarded
                    .into_iter()
                    .map(|value| Stmt {
                        span: value.span,
                        kind: StmtKind::Expression(value),
                    })
                    .collect(),
                value: Box::new(Expr {
                    ty: Type::Void,
                    kind: ExprKind::Zero,
                    span: value.span,
                }),
            }
        };
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        walk_statement(self, statement)?;
        match &mut statement.kind {
            StmtKind::Variable(variable) if variable.ty == Type::Void => {
                statement.kind =
                    variable.initializer.take().map_or(StmtKind::Empty, StmtKind::Expression);
            }
            StmtKind::Variables(variables) if variables.iter().any(|v| v.ty == Type::Void) => {
                statement.kind = StmtKind::Sequence(
                    std::mem::take(variables)
                        .into_iter()
                        .map(|variable| Stmt {
                            span: variable.span,
                            kind: if variable.ty == Type::Void {
                                variable.initializer.map_or(StmtKind::Empty, StmtKind::Expression)
                            } else {
                                StmtKind::Variable(variable)
                            },
                        })
                        .collect(),
                );
            }
            _ => {}
        }
        Ok(())
    }
}
