use crate::*;
use slang_source::Diagnostic;
use std::collections::BTreeSet;

/// Shared traversal for typed transformation passes.
pub(crate) trait Visitor {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic>;

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        walk_statement(self, statement)
    }

    fn module(&mut self, module: &mut Module) -> Result<(), Diagnostic> {
        for global in &mut module.globals {
            if let Some(value) = &mut global.initializer {
                self.expression(value)?;
            }
        }
        for buffer in &mut module.constant_buffers {
            for view in &mut buffer.views {
                if let Some(value) = &mut view.initializer {
                    self.expression(value)?;
                }
            }
        }
        for function in &mut module.functions {
            if let Some(body) = &mut function.body {
                self.statement(body)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn walk_statement<V: Visitor + ?Sized>(
    visitor: &mut V,
    statement: &mut Stmt,
) -> Result<(), Diagnostic> {
    match &mut statement.kind {
        StmtKind::Block(body) | StmtKind::Sequence(body) => {
            for s in body {
                visitor.statement(s)?;
            }
        }
        StmtKind::Variable(v) => {
            if let Some(e) = &mut v.initializer {
                visitor.expression(e)?;
            }
        }
        StmtKind::Variables(values) => {
            for v in values {
                if let Some(e) = &mut v.initializer {
                    visitor.expression(e)?;
                }
            }
        }
        StmtKind::Expression(e) | StmtKind::Return(Some(e)) => visitor.expression(e)?,
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visitor.expression(condition)?;
            visitor.statement(then_branch)?;
            if let Some(branch) = else_branch {
                visitor.statement(branch)?;
            }
        }
        StmtKind::While { condition, body } => {
            visitor.expression(condition)?;
            visitor.statement(body)?;
        }
        StmtKind::DoWhile { body, condition } => {
            visitor.statement(body)?;
            visitor.expression(condition)?;
        }
        StmtKind::For {
            initializer,
            condition,
            increment,
            body,
        } => {
            visitor.statement(initializer)?;
            if let Some(e) = condition {
                visitor.expression(e)?;
            }
            if let Some(e) = increment {
                visitor.expression(e)?;
            }
            visitor.statement(body)?;
        }
        StmtKind::Switch { selector, cases } => {
            visitor.expression(selector)?;
            for case in cases {
                if let Some(e) = &mut case.value {
                    visitor.expression(e)?;
                }
                for s in &mut case.statements {
                    visitor.statement(s)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn walk_expression<V: Visitor + ?Sized>(
    visitor: &mut V,
    expression: &mut Expr,
) -> Result<(), Diagnostic> {
    match &mut expression.kind {
        ExprKind::BlockExpression { statements, value } => {
            for statement in statements {
                visitor.statement(statement)?;
            }
            visitor.expression(value)?;
        }
        ExprKind::Construct(values)
        | ExprKind::Call {
            arguments: values, ..
        }
        | ExprKind::IntrinsicCall {
            arguments: values, ..
        } => {
            for e in values {
                visitor.expression(e)?;
            }
        }
        ExprKind::MethodCall {
            receiver,
            arguments,
            ..
        } => {
            visitor.expression(receiver)?;
            for e in arguments {
                visitor.expression(e)?;
            }
        }
        ExprKind::AddressView(e)
        | ExprKind::ConstantBufferLoad(e)
        | ExprKind::DeclarationDefault(e)
        | ExprKind::Unary { value: e, .. }
        | ExprKind::Cast { value: e }
        | ExprKind::Postfix { value: e, .. } => visitor.expression(e)?,
        ExprKind::Binary { left, right, .. } => {
            visitor.expression(left)?;
            visitor.expression(right)?;
        }
        ExprKind::Member { base, .. } => visitor.expression(base)?,
        ExprKind::Index { base, index } => {
            visitor.expression(base)?;
            visitor.expression(index)?;
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            visitor.expression(condition)?;
            visitor.expression(then_value)?;
            visitor.expression(else_value)?;
        }
        _ => {}
    }
    Ok(())
}

/// Collect declared target bindings without recursing through statement scopes.
pub(crate) fn names(module: &Module) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for s in &module.structures {
        names.insert(s.name.clone());
        names.extend(s.fields.iter().map(|f| f.name.clone()));
    }
    names.extend(module.globals.iter().map(|g| g.name.clone()));
    if let Some(fields) = &module.global_uniform_fields {
        names.extend(fields.iter().map(|f| f.name.clone()));
    }
    names.extend(
        module
            .constant_buffers
            .iter()
            .flat_map(|b| b.views.iter().map(|v| v.name.clone())),
    );
    names.extend(
        module
            .constant_buffers
            .iter()
            .flat_map(|b| b.fields.iter().map(|f| f.name.clone())),
    );
    let mut work = vec![];
    for f in &module.functions {
        names.insert(f.name.clone());
        names.extend(f.parameters.iter().map(|p| p.name.clone()));
        if let Some(body) = &f.body {
            work.push(body);
        }
    }
    while let Some(statement) = work.pop() {
        match &statement.kind {
            StmtKind::Variable(v) => {
                names.insert(v.name.clone());
            }
            StmtKind::Variables(values) => names.extend(values.iter().map(|v| v.name.clone())),
            StmtKind::Block(body) | StmtKind::Sequence(body) => work.extend(body),
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                work.push(then_branch);
                if let Some(branch) = else_branch {
                    work.push(branch);
                }
            }
            StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => work.push(body),
            StmtKind::For {
                initializer, body, ..
            } => {
                work.push(initializer);
                work.push(body);
            }
            StmtKind::Switch { cases, .. } => {
                for case in cases {
                    work.extend(&case.statements);
                }
            }
            _ => {}
        }
    }
    names
}
