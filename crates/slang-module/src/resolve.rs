use crate::link::Names;
use slang_source::{Diagnostic, Span};
use slang_syntax::{Declaration, Expr, ExprId, ExprKind, Stmt, StmtKind, TypeRef, Variable};
use std::collections::BTreeSet;

/// Rewrite module symbols while preserving parameter and block-local bindings.
pub(crate) struct Resolver<'a> {
    own: &'a Names,
    imported: &'a Names,
    scopes: Vec<BTreeSet<String>>,
    next_expression: &'a mut usize,
}

impl<'a> Resolver<'a> {
    pub fn new(own: &'a Names, imported: &'a Names, next_expression: &'a mut usize) -> Self {
        Self {
            own,
            imported,
            scopes: vec![],
            next_expression,
        }
    }

    fn symbol(&self, name: &str, is_type: bool, span: Span) -> Result<String, Diagnostic> {
        if let Some(target) = self.own.lookup(name, is_type, span)? {
            return Ok(target);
        }
        Ok(self
            .imported
            .lookup(name, is_type, span)?
            .unwrap_or_else(|| name.into()))
    }

    fn ty(&self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        ty.name = self.symbol(&ty.name, true, ty.span)?;
        for argument in &mut ty.arguments {
            self.ty(argument)?;
        }
        Ok(())
    }

    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        self.ty(&mut variable.ty)?;
        if let Some(value) = &mut variable.initializer {
            self.expression(value)?;
        }
        Ok(())
    }

    pub fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        match declaration {
            Declaration::TypeAlias(alias) => {
                alias.name = self.symbol(&alias.name, true, alias.span)?;
                self.ty(&mut alias.target)?;
            }
            Declaration::Struct(structure) => {
                structure.name = self.symbol(&structure.name, true, structure.span)?;
                for field in &mut structure.fields {
                    self.variable(field)?;
                }
            }
            Declaration::Global(variable) => {
                self.variable(variable)?;
                variable.name = self.symbol(&variable.name, false, variable.span)?;
            }
            Declaration::Function(function) => {
                function.name = self.symbol(&function.name, false, function.span)?;
                self.ty(&mut function.return_type)?;
                self.scopes
                    .push(function.parameters.iter().map(|p| p.name.clone()).collect());
                for parameter in &mut function.parameters {
                    self.variable(parameter)?;
                }
                for attribute in &mut function.attributes {
                    for argument in &mut attribute.arguments {
                        self.expression(argument)?;
                    }
                }
                if let Some(body) = &mut function.body {
                    self.statement(body)?;
                }
                self.scopes.pop();
            }
        }
        Ok(())
    }

    fn scoped_statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        self.scopes.push(BTreeSet::new());
        self.statement(statement)?;
        self.scopes.pop();
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(statements) => {
                self.scopes.push(BTreeSet::new());
                for statement in statements {
                    self.statement(statement)?;
                }
                self.scopes.pop();
            }
            StmtKind::Variable(variable) => {
                self.variable(variable)?;
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(variable.name.clone());
            }
            StmtKind::Variables(variables) => {
                for variable in variables {
                    self.variable(variable)?;
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(variable.name.clone());
                }
            }
            StmtKind::Expression(value) => self.expression(value)?,
            StmtKind::Return(Some(value)) => self.expression(value)?,
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression(condition)?;
                self.scoped_statement(then_branch)?;
                if let Some(branch) = else_branch {
                    self.scoped_statement(branch)?;
                }
            }
            StmtKind::While { condition, body } => {
                self.expression(condition)?;
                self.scoped_statement(body)?;
            }
            StmtKind::DoWhile { condition, body } => {
                self.scoped_statement(body)?;
                self.expression(condition)?;
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(BTreeSet::new());
                self.statement(initializer)?;
                if let Some(condition) = condition {
                    self.expression(condition)?;
                }
                if let Some(increment) = increment {
                    self.expression(increment)?;
                }
                self.scoped_statement(body)?;
                self.scopes.pop();
            }
            StmtKind::Switch { selector, cases } => {
                self.expression(selector)?;
                self.scopes.push(BTreeSet::new());
                for case in cases {
                    if let Some(value) = &mut case.value {
                        self.expression(value)?;
                    }
                    for statement in &mut case.statements {
                        self.statement(statement)?;
                    }
                }
                self.scopes.pop();
            }
            StmtKind::Break | StmtKind::Continue | StmtKind::Return(None) | StmtKind::Empty => {}
        }
        Ok(())
    }

    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        expression.id = ExprId(*self.next_expression);
        *self.next_expression += 1;
        match &mut expression.kind {
            ExprKind::InitializerList(values) => {
                for value in values {
                    self.expression(value)?;
                }
            }
            ExprKind::Name(name) => {
                if !self.scopes.iter().rev().any(|scope| scope.contains(name)) {
                    if let Some(target) = self.own.lookup(name, false, expression.span)? {
                        *name = target;
                    } else if let Some(target) = self.own.lookup(name, true, expression.span)? {
                        *name = target;
                    } else if let Some(target) =
                        self.imported.lookup(name, false, expression.span)?
                    {
                        *name = target;
                    } else if let Some(target) =
                        self.imported.lookup(name, true, expression.span)?
                    {
                        *name = target;
                    }
                }
            }
            ExprKind::Cast { target, value } => {
                self.ty(target)?;
                self.expression(value)?;
            }
            ExprKind::Unary { value, .. } | ExprKind::Postfix { value, .. } => {
                self.expression(value)?
            }
            ExprKind::Binary { left, right, .. } => {
                self.expression(left)?;
                self.expression(right)?;
            }
            ExprKind::Call { callee, arguments } => {
                self.expression(callee)?;
                for argument in arguments {
                    self.expression(argument)?;
                }
            }
            ExprKind::Member { base, .. } => self.expression(base)?,
            ExprKind::Index { base, index } => {
                self.expression(base)?;
                self.expression(index)?;
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.expression(condition)?;
                self.expression(then_value)?;
                self.expression(else_value)?;
            }
            ExprKind::Integer(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::String(_) => {
            }
        }
        Ok(())
    }
}
