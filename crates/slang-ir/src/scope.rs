use crate::*;

/// HLSL control bodies need braces, but unbraced Slang declarations belong to
/// the surrounding source scope. Keep storage there and execution in the body.
pub(crate) fn lower(module: &mut Module) {
    for function in &mut module.functions {
        if let Some(body) = &mut function.body {
            let mut declarations = vec![];
            statement(body, false, &mut declarations);
            debug_assert!(declarations.is_empty());
        }
    }
}

fn sequence(statements: &mut Vec<Stmt>) {
    let mut result = vec![];
    for mut value in std::mem::take(statements) {
        statement(&mut value, false, &mut result);
        result.push(value);
    }
    *statements = result;
}

fn statement(value: &mut Stmt, controlled: bool, declarations: &mut Vec<Stmt>) {
    match &mut value.kind {
        StmtKind::Block(body) => sequence(body),
        StmtKind::Sequence(body) => {
            if controlled {
                for value in body {
                    statement(value, true, declarations);
                }
            } else {
                sequence(body);
            }
        }
        StmtKind::Variable(variable) if controlled => {
            value.kind = initialize(variable, declarations);
        }
        StmtKind::Variables(variables) if controlled => {
            value.kind = StmtKind::Sequence(
                variables
                    .iter_mut()
                    .map(|variable| Stmt {
                        span: variable.span,
                        kind: initialize(variable, declarations),
                    })
                    .collect(),
            );
        }
        StmtKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            statement(then_branch, true, declarations);
            if let Some(branch) = else_branch {
                statement(branch, true, declarations);
            }
        }
        StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => {
            statement(body, true, declarations);
        }
        StmtKind::For { body, .. } => {
            let mut locals = vec![];
            statement(body, true, &mut locals);
            if !locals.is_empty() {
                locals.push(value.clone());
                value.kind = StmtKind::Block(locals);
            }
        }
        StmtKind::Switch { cases, .. } => {
            for case in cases {
                sequence(&mut case.statements);
            }
        }
        _ => {}
    }
}

fn initialize(variable: &mut Variable, declarations: &mut Vec<Stmt>) -> StmtKind {
    let constant = variable.modifiers.iter().any(|m| m == "const")
        && variable.initializer.as_ref().is_some_and(literal_constant);
    if constant {
        declarations.push(Stmt {
            span: variable.span,
            kind: StmtKind::Variable(variable.clone()),
        });
        return StmtKind::Empty;
    }
    let initializer = variable.initializer.take();
    variable.modifiers.retain(|m| m != "const");
    declarations.push(Stmt {
        span: variable.span,
        kind: StmtKind::Variable(variable.clone()),
    });
    initializer.map_or(StmtKind::Empty, |right| {
        StmtKind::Expression(Expr {
            ty: variable.ty.clone(),
            span: variable.span,
            kind: ExprKind::Binary {
                op: "=".into(),
                left: Box::new(Expr {
                    kind: ExprKind::Name(variable.name.clone()),
                    ty: variable.ty.clone(),
                    span: variable.span,
                }),
                right: Box::new(right),
            },
        })
    })
}

/// Pure literal constants have a value even when the declaration is unreachable.
fn literal_constant(value: &Expr) -> bool {
    match &value.kind {
        ExprKind::Integer(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Zero => true,
        ExprKind::Construct(values) => values.iter().all(literal_constant),
        ExprKind::Call { name, arguments }
            if slang_sema::Type::builtin(name).as_ref() == Some(&value.ty) =>
        {
            arguments.iter().all(literal_constant)
        }
        ExprKind::Cast { value } | ExprKind::Unary { value, .. } => literal_constant(value),
        ExprKind::Binary { op, left, right } => {
            !matches!(
                op.as_str(),
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
            ) && literal_constant(left)
                && literal_constant(right)
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            literal_constant(condition)
                && literal_constant(then_value)
                && literal_constant(else_value)
        }
        _ => false,
    }
}
