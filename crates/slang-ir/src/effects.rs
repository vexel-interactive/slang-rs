//! Eliminate statement expressions while preserving source evaluation points.

use crate::*;
use slang_sema::{CheckedModule, Scalar, Type, span_key};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

pub(crate) fn lower(module: &mut Module, checked: &CheckedModule) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_effect_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let signatures = module
        .functions
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                f.parameters
                    .iter()
                    .map(|p| {
                        p.modifiers
                            .iter()
                            .any(|m| matches!(m.as_str(), "out" | "inout"))
                    })
                    .collect(),
            )
        })
        .collect();
    let mut lowerer = Lowerer {
        prefix,
        counter: 0,
        declarations: vec![],
        signatures,
        checked,
    };
    for function in &mut module.functions {
        if let Some(body) = &mut function.body {
            lowerer.statement(body)?;
            if !lowerer.declarations.is_empty() {
                let mut declarations = std::mem::take(&mut lowerer.declarations);
                declarations.push(body.clone());
                body.kind = StmtKind::Block(declarations);
            }
        }
    }
    // Captures can include aggregate constructors in surrounding expressions.
    crate::initialization::lower(module)?;
    crate::scope::lower(module);
    Ok(())
}

struct Lowerer<'a> {
    prefix: String,
    counter: usize,
    declarations: Vec<Stmt>,
    signatures: BTreeMap<String, Vec<bool>>,
    checked: &'a CheckedModule,
}

impl Lowerer<'_> {
    fn storage(&mut self, ty: &Type, span: Span) -> Expr {
        let name = format!("{}{}", self.prefix, self.counter);
        self.counter += 1;
        self.declarations.push(Stmt {
            kind: StmtKind::Variable(variable(name.clone(), ty.clone(), span)),
            span,
        });
        Expr {
            kind: ExprKind::Name(name),
            ty: ty.clone(),
            span,
        }
    }

    fn save(&mut self, value: &mut Expr, prefix: &mut Vec<Stmt>) {
        // Literal values cannot change across a later statement region.
        if matches!(
            value.kind,
            ExprKind::Integer(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Zero
        ) {
            return;
        }
        let local = self.storage(&value.ty, value.span);
        prefix.push(expression(assign(local.clone(), value.clone())));
        *value = local;
    }

    fn outputs(&self, name: &str, span: Span, count: usize, ordinary: bool) -> Vec<bool> {
        let signature = if ordinary {
            self.signatures.get(name)
        } else {
            None
        };
        let mut outputs = signature
            .or_else(|| {
                self.checked
                    .call_output_parameters
                    .get(&span_key(span))
                    .filter(|(callee, _)| callee == name)
                    .map(|(_, outputs)| outputs)
            })
            .cloned()
            .unwrap_or_else(|| vec![false; count]);
        // Atomic lowering can append a private discarded output argument.
        outputs.resize(count, true);
        outputs
    }

    fn arguments(
        &mut self,
        values: &mut [Expr],
        outputs: &[bool],
    ) -> Result<Vec<Stmt>, Diagnostic> {
        let mut prefixes = vec![];
        for (i, value) in values.iter_mut().enumerate() {
            prefixes.push(if outputs.get(i).copied().unwrap_or(false) {
                self.location(value, false)?
            } else {
                self.value(value)?
            });
        }
        let mut prefix = vec![];
        for (i, value) in values.iter_mut().enumerate() {
            prefix.append(&mut prefixes[i]);
            if prefixes[i + 1..].iter().any(|p| !p.is_empty()) {
                if outputs.get(i).copied().unwrap_or(false) {
                    prefix.extend(self.location(value, true)?);
                } else {
                    self.save(value, &mut prefix);
                }
            }
        }
        Ok(prefix)
    }

    /// Preserve a storage path while capturing its evaluated coordinates. The
    /// value stored at a compound destination is read only after the RHS.
    fn location(&mut self, value: &mut Expr, freeze: bool) -> Result<Vec<Stmt>, Diagnostic> {
        match &mut value.kind {
            ExprKind::Name(_) | ExprKind::GlobalName(_) => Ok(vec![]),
            ExprKind::Member { base, .. } => {
                if storage_path(base) && !base.ty.is_resource() {
                    self.location(base, freeze)
                } else {
                    let mut prefix = self.value(base)?;
                    if freeze {
                        self.save(base, &mut prefix);
                    }
                    Ok(prefix)
                }
            }
            ExprKind::Index { base, index } => {
                let is_storage = storage_path(base) && !base.ty.is_resource();
                let mut prefix = if is_storage {
                    self.location(base, false)?
                } else {
                    self.value(base)?
                };
                let mut index_prefix = self.value(index)?;
                if freeze || !index_prefix.is_empty() {
                    if is_storage {
                        prefix.extend(self.location(base, true)?);
                    } else {
                        self.save(base, &mut prefix);
                    }
                }
                prefix.append(&mut index_prefix);
                if freeze {
                    self.save(index, &mut prefix);
                }
                Ok(prefix)
            }
            _ => {
                let mut prefix = self.value(value)?;
                if freeze {
                    self.save(value, &mut prefix);
                }
                Ok(prefix)
            }
        }
    }

    fn value(&mut self, value: &mut Expr) -> Result<Vec<Stmt>, Diagnostic> {
        let span = value.span;
        let ty = value.ty.clone();
        match &mut value.kind {
            ExprKind::BlockExpression {
                statements,
                value: result,
            } => {
                for statement in statements.iter_mut() {
                    self.statement(statement)?;
                }
                let mut prefix = std::mem::take(statements);
                prefix.extend(self.value(result)?);
                *value = *result.clone();
                Ok(prefix)
            }
            ExprKind::Construct(values) => self.arguments(values, &[]),
            ExprKind::Call { name, arguments } => {
                let outputs = self.outputs(name, span, arguments.len(), true);
                self.arguments(arguments, &outputs)
            }
            ExprKind::IntrinsicCall { name, arguments } => {
                let outputs = self.outputs(name, span, arguments.len(), false);
                self.arguments(arguments, &outputs)
            }
            ExprKind::MethodCall {
                receiver,
                name,
                arguments,
            } => {
                let mut prefix = self.value(receiver)?;
                let outputs = self.outputs(name, span, arguments.len(), false);
                let mut argument_prefix = self.arguments(arguments, &outputs)?;
                if !argument_prefix.is_empty() {
                    self.save(receiver, &mut prefix);
                }
                prefix.append(&mut argument_prefix);
                Ok(prefix)
            }
            ExprKind::Unary { op, value } | ExprKind::Postfix { op, value }
                if matches!(op.as_str(), "++" | "--") =>
            {
                self.location(value, false)
            }
            ExprKind::Unary { value, .. }
            | ExprKind::Cast { value }
            | ExprKind::ConstantBufferLoad(value)
            | ExprKind::DeclarationDefault(value) => self.value(value),
            ExprKind::Member { .. } | ExprKind::Index { .. } => self.location(value, false),
            ExprKind::Binary { op, left, right } if assignment(op) => {
                let mut prefix = self.location(left, false)?;
                let mut right_prefix = self.value(right)?;
                if !right_prefix.is_empty() {
                    prefix.extend(self.location(left, true)?);
                }
                prefix.append(&mut right_prefix);
                Ok(prefix)
            }
            ExprKind::Binary { op, left, right } if matches!(op.as_str(), "&&" | "||") => {
                let mut prefix = self.value(left)?;
                let mut right_prefix = self.value(right)?;
                if right_prefix.is_empty() {
                    return Ok(prefix);
                }
                if !matches!(left.ty, Type::Scalar(_)) {
                    return Err(Diagnostic::error(
                        "I005",
                        span,
                        "statement-valued vector logical operands are not implemented",
                    ));
                }
                let result = self.storage(&Type::Scalar(Scalar::Bool), span);
                prefix.push(expression(assign(result.clone(), *left.clone())));
                let condition = if op == "&&" {
                    result.clone()
                } else {
                    negate(result.clone())
                };
                right_prefix.push(expression(assign(result.clone(), *right.clone())));
                prefix.push(guard(condition, block(right_prefix, span)));
                *value = result;
                Ok(prefix)
            }
            ExprKind::Binary { op, left, right } => {
                let mut prefix = self.value(left)?;
                let mut right_prefix = self.value(right)?;
                if op == "," && no_value(left) {
                    prefix.append(&mut right_prefix);
                    *value = *right.clone();
                    return Ok(prefix);
                }
                if !right_prefix.is_empty() {
                    if op == "," {
                        prefix.push(expression(*left.clone()));
                        prefix.append(&mut right_prefix);
                        *value = *right.clone();
                        return Ok(prefix);
                    }
                    self.save(left, &mut prefix);
                }
                prefix.append(&mut right_prefix);
                if op == "," && no_value(right) {
                    prefix.push(expression(*left.clone()));
                    *value = *right.clone();
                }
                Ok(prefix)
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let mut prefix = self.value(condition)?;
                let mut then_prefix = self.value(then_value)?;
                let mut else_prefix = self.value(else_value)?;
                if then_prefix.is_empty() && else_prefix.is_empty() {
                    return Ok(prefix);
                }
                if !matches!(condition.ty, Type::Scalar(_)) {
                    return Err(Diagnostic::error(
                        "I005",
                        span,
                        "statement-valued vector conditional arms are not implemented",
                    ));
                }
                let result = if ty == Type::Void {
                    then_prefix.push(expression(*then_value.clone()));
                    else_prefix.push(expression(*else_value.clone()));
                    Expr {
                        kind: ExprKind::Zero,
                        ty,
                        span,
                    }
                } else {
                    let result = self.storage(&ty, span);
                    then_prefix.push(expression(assign(result.clone(), *then_value.clone())));
                    else_prefix.push(expression(assign(result.clone(), *else_value.clone())));
                    result
                };
                prefix.push(Stmt {
                    span,
                    kind: StmtKind::If {
                        condition: *condition.clone(),
                        then_branch: Box::new(block(then_prefix, span)),
                        else_branch: Some(Box::new(block(else_prefix, span))),
                    },
                });
                *value = result;
                Ok(prefix)
            }
            _ => Ok(vec![]),
        }
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        let span = statement.span;
        let prefix = match &mut statement.kind {
            StmtKind::Block(values) | StmtKind::Sequence(values) => {
                for value in values {
                    self.statement(value)?;
                }
                vec![]
            }
            StmtKind::Variable(variable) => variable
                .initializer
                .as_mut()
                .map(|v| self.value(v))
                .transpose()?
                .unwrap_or_default(),
            StmtKind::Variables(variables) => {
                let mut values = vec![];
                for variable in std::mem::take(variables) {
                    let mut value = Stmt {
                        kind: StmtKind::Variable(variable),
                        span,
                    };
                    self.statement(&mut value)?;
                    values.push(value);
                }
                statement.kind = StmtKind::Sequence(values);
                vec![]
            }
            StmtKind::Expression(value) | StmtKind::Return(Some(value)) => self.value(value)?,
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let prefix = self.value(condition)?;
                self.statement(then_branch)?;
                if let Some(branch) = else_branch {
                    self.statement(branch)?;
                }
                prefix
            }
            StmtKind::Switch { selector, cases } => {
                let prefix = self.value(selector)?;
                for case in cases {
                    for value in &mut case.statements {
                        self.statement(value)?;
                    }
                }
                prefix
            }
            StmtKind::While { condition, body } => {
                let mut prefix = self.value(condition)?;
                self.statement(body)?;
                if !prefix.is_empty() {
                    prefix.push(guard(
                        negate(condition.clone()),
                        Stmt {
                            kind: StmtKind::Break,
                            span,
                        },
                    ));
                    prefix.push(*body.clone());
                    *condition = boolean(true, span);
                    **body = block(prefix, span);
                }
                vec![]
            }
            StmtKind::DoWhile { body, condition } => {
                self.statement(body)?;
                let mut condition_prefix = self.value(condition)?;
                if !condition_prefix.is_empty() {
                    let first = self.storage(&Type::Scalar(Scalar::Bool), span);
                    condition_prefix.push(guard(
                        negate(condition.clone()),
                        Stmt {
                            kind: StmtKind::Break,
                            span,
                        },
                    ));
                    let loop_body = block(
                        vec![
                            guard(negate(first.clone()), block(condition_prefix, span)),
                            expression(assign(first.clone(), boolean(false, span))),
                            *body.clone(),
                        ],
                        span,
                    );
                    statement.kind = StmtKind::Sequence(vec![
                        expression(assign(first, boolean(true, span))),
                        Stmt {
                            span,
                            kind: StmtKind::While {
                                condition: boolean(true, span),
                                body: Box::new(loop_body),
                            },
                        },
                    ]);
                }
                vec![]
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.statement(initializer)?;
                let mut condition_prefix = condition
                    .as_mut()
                    .map(|v| self.value(v))
                    .transpose()?
                    .unwrap_or_default();
                let mut increment_prefix = increment
                    .as_mut()
                    .map(|v| self.value(v))
                    .transpose()?
                    .unwrap_or_default();
                self.statement(body)?;
                if !condition_prefix.is_empty() || !increment_prefix.is_empty() {
                    let first = self.storage(&Type::Scalar(Scalar::Bool), span);
                    if let Some(increment) = increment {
                        increment_prefix.push(expression(increment.clone()));
                    }
                    if let Some(condition) = condition {
                        condition_prefix.push(guard(
                            negate(condition.clone()),
                            Stmt {
                                kind: StmtKind::Break,
                                span,
                            },
                        ));
                    }
                    let mut values = vec![
                        guard(negate(first.clone()), block(increment_prefix, span)),
                        expression(assign(first.clone(), boolean(false, span))),
                    ];
                    values.append(&mut condition_prefix);
                    values.push(*body.clone());
                    statement.kind = StmtKind::Block(vec![
                        *initializer.clone(),
                        expression(assign(first, boolean(true, span))),
                        Stmt {
                            span,
                            kind: StmtKind::While {
                                condition: boolean(true, span),
                                body: Box::new(block(values, span)),
                            },
                        },
                    ]);
                }
                vec![]
            }
            _ => vec![],
        };
        match &statement.kind {
            StmtKind::Expression(value) if no_value(value) => statement.kind = StmtKind::Empty,
            StmtKind::Return(Some(value)) if no_value(value) => {
                statement.kind = StmtKind::Return(None)
            }
            _ => {}
        }
        if !prefix.is_empty() {
            let mut prefix = prefix;
            prefix.push(statement.clone());
            statement.kind = StmtKind::Sequence(prefix);
        }
        Ok(())
    }
}

fn storage_path(value: &Expr) -> bool {
    match &value.kind {
        ExprKind::Name(_) | ExprKind::GlobalName(_) => true,
        ExprKind::Index { base, .. } | ExprKind::Member { base, .. } => storage_path(base),
        _ => false,
    }
}

fn assignment(op: &str) -> bool {
    matches!(
        op,
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
    )
}

pub(super) fn variable(name: String, ty: Type, span: Span) -> Variable {
    Variable {
        name,
        ty,
        modifiers: vec![],
        semantic: None,
        binding: None,
        uniform_layout: None,
        pack_offset: None,
        initializer: None,
        span,
    }
}

pub(super) fn assign(left: Expr, mut right: Expr) -> Expr {
    if left.ty != right.ty {
        right = Expr {
            ty: left.ty.clone(),
            span: right.span,
            kind: ExprKind::Cast {
                value: Box::new(right),
            },
        };
    }
    binary("=", left, right)
}

pub(super) fn binary(op: &str, left: Expr, right: Expr) -> Expr {
    Expr {
        ty: if op == "," {
            right.ty.clone()
        } else {
            left.ty.clone()
        },
        span: left.span,
        kind: ExprKind::Binary {
            op: op.into(),
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

pub(super) fn boolean(value: bool, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Bool(value),
        ty: Type::Scalar(Scalar::Bool),
        span,
    }
}

pub(super) fn negate(value: Expr) -> Expr {
    Expr {
        ty: Type::Scalar(Scalar::Bool),
        span: value.span,
        kind: ExprKind::Unary {
            op: "!".into(),
            value: Box::new(value),
        },
    }
}

pub(super) fn expression(value: Expr) -> Stmt {
    Stmt {
        span: value.span,
        kind: if no_value(&value) {
            StmtKind::Empty
        } else {
            StmtKind::Expression(value)
        },
    }
}

fn no_value(value: &Expr) -> bool {
    value.ty == Type::Void && matches!(value.kind, ExprKind::Zero)
}

pub(super) fn block(values: Vec<Stmt>, span: Span) -> Stmt {
    Stmt {
        kind: StmtKind::Block(values),
        span,
    }
}

pub(super) fn guard(condition: Expr, body: Stmt) -> Stmt {
    Stmt {
        span: body.span,
        kind: StmtKind::If {
            condition,
            then_branch: Box::new(body),
            else_branch: None,
        },
    }
}
