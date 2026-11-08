use super::*;
use crate::effects::{assign, binary, block, boolean, expression, guard, negate, variable};
use slang_sema::Scalar;

impl Lowerer {
    pub(super) fn expand_control(
        &mut self,
        function: &Function,
        arguments: &[Expr],
        span: Span,
    ) -> Result<Expr, Diagnostic> {
        if arguments.len() != function.parameters.len() {
            return Err(unsupported(
                span,
                "ForceInline argument count does not match checked parameters",
            ));
        }
        self.active.push(function.name.clone());
        let mut statements = vec![];
        let mut bindings = BTreeMap::new();
        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            let local = self.storage(parameter, span);
            statements.push(expression(assign(local.clone(), argument.clone())));
            bindings.insert(parameter.name.clone(), local);
        }
        let result = if function.return_type == Type::Void {
            Expr {
                ty: Type::Void,
                kind: ExprKind::Zero,
                span,
            }
        } else {
            self.storage(
                &variable(String::new(), function.return_type.clone(), span),
                span,
            )
        };
        let returned = self.storage(
            &variable(String::new(), Type::Scalar(Scalar::Bool), span),
            span,
        );
        statements.push(expression(assign(returned.clone(), boolean(false, span))));
        statements.push(self.control(
            function.body.as_ref().unwrap(),
            &mut bindings,
            &result,
            &returned,
        )?);
        let mut value = Expr {
            ty: function.return_type.clone(),
            span,
            kind: ExprKind::BlockExpression {
                statements,
                value: Box::new(result),
            },
        };
        self.expression(&mut value)?;
        self.active.pop();
        Ok(value)
    }

    /// Lexical binding maps are copied only at source scope boundaries. All
    /// capture storage has unique names, while guarded initializers execute at
    /// the original declaration site on every loop iteration.
    fn control(
        &mut self,
        statement: &Stmt,
        bindings: &mut BTreeMap<String, Expr>,
        result: &Expr,
        returned: &Expr,
    ) -> Result<Stmt, Diagnostic> {
        let span = statement.span;
        let kind = match &statement.kind {
            StmtKind::Block(values) | StmtKind::Sequence(values) => {
                let scoped = matches!(statement.kind, StmtKind::Block(_));
                let saved = scoped.then(|| bindings.clone());
                let mut statements = vec![];
                for value in values {
                    statements.push(self.control(value, bindings, result, returned)?);
                }
                if let Some(saved) = saved {
                    *bindings = saved;
                }
                return Ok(if scoped {
                    block(statements, span)
                } else {
                    Stmt {
                        span,
                        kind: StmtKind::Sequence(statements),
                    }
                });
            }
            StmtKind::Variable(value) => {
                let mut expressions = vec![];
                self.local(value, bindings, &mut expressions, span)?;
                StmtKind::Sequence(expressions.into_iter().map(expression).collect())
            }
            StmtKind::Variables(values) => {
                let mut expressions = vec![];
                for value in values {
                    self.local(value, bindings, &mut expressions, span)?;
                }
                StmtKind::Sequence(expressions.into_iter().map(expression).collect())
            }
            StmtKind::Expression(value) => StmtKind::Expression(capture(value, bindings)?),
            StmtKind::Return(Some(value)) => StmtKind::Block(vec![
                expression(if result.ty == Type::Void {
                    capture(value, bindings)?
                } else {
                    assign(result.clone(), capture(value, bindings)?)
                }),
                expression(assign(returned.clone(), boolean(true, span))),
            ]),
            StmtKind::Return(None) => {
                if result.ty != Type::Void {
                    return Err(unsupported(span, "ForceInline requires a return value"));
                }
                StmtKind::Expression(assign(returned.clone(), boolean(true, span)))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => StmtKind::If {
                condition: capture(condition, bindings)?,
                then_branch: Box::new(self.control(then_branch, bindings, result, returned)?),
                else_branch: else_branch
                    .as_ref()
                    .map(|branch| {
                        self.control(branch, bindings, result, returned)
                            .map(Box::new)
                    })
                    .transpose()?,
            },
            StmtKind::While { condition, body } => StmtKind::While {
                condition: binary(
                    "&&",
                    negate(returned.clone()),
                    capture(condition, bindings)?,
                ),
                body: Box::new(self.control(body, bindings, result, returned)?),
            },
            StmtKind::DoWhile { body, condition } => {
                let body = self.control(body, bindings, result, returned)?;
                StmtKind::DoWhile {
                    body: Box::new(body),
                    condition: binary(
                        "&&",
                        negate(returned.clone()),
                        capture(condition, bindings)?,
                    ),
                }
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                let saved = bindings.clone();
                let initializer = self.control(initializer, bindings, result, returned)?;
                let condition = binary(
                    "&&",
                    negate(returned.clone()),
                    condition
                        .as_ref()
                        .map(|c| capture(c, bindings))
                        .transpose()?
                        .unwrap_or_else(|| boolean(true, span)),
                );
                let increment = increment
                    .as_ref()
                    .map(|value| {
                        let zero = Expr {
                            kind: ExprKind::Integer("0".into()),
                            ty: Type::Scalar(Scalar::Int),
                            span,
                        };
                        Ok::<_, Diagnostic>(Expr {
                            ty: zero.ty.clone(),
                            span,
                            kind: ExprKind::Conditional {
                                condition: Box::new(returned.clone()),
                                then_value: Box::new(zero.clone()),
                                else_value: Box::new(binary(",", capture(value, bindings)?, zero)),
                            },
                        })
                    })
                    .transpose()?;
                let body = self.control(body, bindings, result, returned)?;
                *bindings = saved;
                StmtKind::For {
                    initializer: Box::new(Stmt {
                        span,
                        kind: StmtKind::Sequence(vec![initializer]),
                    }),
                    condition: Some(condition),
                    increment,
                    body: Box::new(body),
                }
            }
            StmtKind::Switch { selector, cases } => {
                let selector = capture(selector, bindings)?;
                let saved = bindings.clone();
                let mut values = vec![];
                for case in cases {
                    let value = case
                        .value
                        .as_ref()
                        .map(|value| capture(value, bindings))
                        .transpose()?;
                    let mut statements = vec![];
                    for value in &case.statements {
                        statements.push(self.control(value, bindings, result, returned)?);
                    }
                    values.push(SwitchCase { value, statements });
                }
                *bindings = saved;
                StmtKind::Switch {
                    selector,
                    cases: values,
                }
            }
            StmtKind::Break => StmtKind::Break,
            StmtKind::Continue => StmtKind::Continue,
            StmtKind::Discard => StmtKind::Discard,
            StmtKind::Empty => return Ok(statement.clone()),
        };
        Ok(guard(negate(returned.clone()), Stmt { span, kind }))
    }
}
