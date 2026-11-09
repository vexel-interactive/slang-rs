//! Mutable AST traversal for transformations that preserve source locations.

use crate::*;
use slang_source::{Diagnostic, Span};

pub trait VisitorMut: Sized {
    fn span(&mut self, _span: &mut Span) {}
    fn expression_id(&mut self, _id: &mut ExprId) {}
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        walk_type(self, ty)
    }
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)
    }
    fn variable(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        walk_variable(self, variable)
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        walk_statement(self, statement)
    }
    fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        walk_declaration(self, declaration)
    }
}

pub fn walk_type(visitor: &mut impl VisitorMut, ty: &mut TypeRef) -> Result<(), Diagnostic> {
    visitor.span(&mut ty.span);
    if let Some(base) = &mut ty.base {
        visitor.ty(base)?;
    }
    for argument in &mut ty.arguments {
        match argument {
            TypeArgument::Type(ty) => visitor.ty(ty)?,
            TypeArgument::Expression(value) => visitor.expression(value)?,
            TypeArgument::Integer { span, .. } => visitor.span(span),
        }
    }
    for dimension in &mut ty.arrays {
        visitor.expression(dimension)?;
    }
    Ok(())
}

pub fn walk_expression(
    visitor: &mut impl VisitorMut,
    expression: &mut Expr,
) -> Result<(), Diagnostic> {
    visitor.span(&mut expression.span);
    visitor.expression_id(&mut expression.id);
    match &mut expression.kind {
        ExprKind::InitializerList(values) => {
            for value in values {
                visitor.expression(value)?;
            }
        }
        ExprKind::TypeApplication(ty) => visitor.ty(ty)?,
        ExprKind::GenericApplication { callee, arguments } => {
            visitor.expression(callee)?;
            for argument in arguments {
                match argument {
                    TypeArgument::Type(ty) => visitor.ty(ty)?,
                    TypeArgument::Expression(value) => visitor.expression(value)?,
                    TypeArgument::Integer { span, .. } => visitor.span(span),
                }
            }
        }
        ExprKind::Unary { value, .. } | ExprKind::Postfix { value, .. } => {
            visitor.expression(value)?
        }
        ExprKind::Cast { target, value } => {
            visitor.ty(target)?;
            visitor.expression(value)?;
        }
        ExprKind::Binary { left, right, .. } => {
            visitor.expression(left)?;
            visitor.expression(right)?;
        }
        ExprKind::Call { callee, arguments } => {
            visitor.expression(callee)?;
            for argument in arguments {
                visitor.expression(argument)?;
            }
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
        ExprKind::Integer(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Name(_) => {}
    }
    Ok(())
}

pub fn walk_variable(
    visitor: &mut impl VisitorMut,
    variable: &mut Variable,
) -> Result<(), Diagnostic> {
    visitor.span(&mut variable.span);
    visitor.ty(&mut variable.ty)?;
    if let Some(register) = &mut variable.register {
        visitor.span(&mut register.span);
    }
    if let Some(pack) = &mut variable.pack_offset {
        visitor.span(&mut pack.span);
    }
    if let Some(initializer) = &mut variable.initializer {
        visitor.expression(initializer)?;
    }
    Ok(())
}

pub fn walk_statement(
    visitor: &mut impl VisitorMut,
    statement: &mut Stmt,
) -> Result<(), Diagnostic> {
    visitor.span(&mut statement.span);
    match &mut statement.kind {
        StmtKind::Block(statements) => {
            for statement in statements {
                visitor.statement(statement)?;
            }
        }
        StmtKind::Variable(variable) => visitor.variable(variable)?,
        StmtKind::Variables(variables) => {
            for variable in variables {
                visitor.variable(variable)?;
            }
        }
        StmtKind::Expression(value) | StmtKind::Return(Some(value)) => visitor.expression(value)?,
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
        StmtKind::While { condition, body } | StmtKind::DoWhile { condition, body } => {
            visitor.expression(condition)?;
            visitor.statement(body)?;
        }
        StmtKind::For {
            initializer,
            condition,
            increment,
            body,
        } => {
            visitor.statement(initializer)?;
            if let Some(condition) = condition {
                visitor.expression(condition)?;
            }
            if let Some(increment) = increment {
                visitor.expression(increment)?;
            }
            visitor.statement(body)?;
        }
        StmtKind::Switch { selector, cases } => {
            visitor.expression(selector)?;
            for case in cases {
                visitor.span(&mut case.span);
                if let Some(value) = &mut case.value {
                    visitor.expression(value)?;
                }
                for statement in &mut case.statements {
                    visitor.statement(statement)?;
                }
            }
        }
        StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Discard
        | StmtKind::Return(None)
        | StmtKind::Empty => {}
    }
    Ok(())
}

pub fn walk_declaration(
    visitor: &mut impl VisitorMut,
    declaration: &mut Declaration,
) -> Result<(), Diagnostic> {
    match declaration {
        Declaration::TypeAlias(alias) => {
            visitor.span(&mut alias.span);
            walk_parameters(visitor, &mut alias.generic_parameters)?;
            visitor.ty(&mut alias.target)?;
        }
        Declaration::Struct(structure) => {
            visitor.span(&mut structure.span);
            walk_parameters(visitor, &mut structure.generic_parameters)?;
            for base in &mut structure.bases {
                visitor.ty(base)?;
            }
            for field in &mut structure.fields {
                visitor.variable(field)?;
            }
        }
        Declaration::Interface(interface) => {
            visitor.span(&mut interface.span);
            walk_parameters(visitor, &mut interface.generic_parameters)?;
            for base in &mut interface.bases {
                visitor.ty(base)?;
            }
            for requirement in &mut interface.requirements {
                walk_function(visitor, requirement)?;
            }
        }
        Declaration::Global(variable) => visitor.variable(variable)?,
        Declaration::Function(function) => {
            walk_function(visitor, function)?;
        }
    }
    Ok(())
}

pub fn walk_function(
    visitor: &mut impl VisitorMut,
    function: &mut Function,
) -> Result<(), Diagnostic> {
    visitor.span(&mut function.span);
    walk_parameters(visitor, &mut function.generic_parameters)?;
    visitor.ty(&mut function.return_type)?;
    for attribute in &mut function.attributes {
        visitor.span(&mut attribute.span);
        for argument in &mut attribute.arguments {
            visitor.expression(argument)?;
        }
    }
    for parameter in &mut function.parameters {
        visitor.variable(parameter)?;
    }
    if let Some(body) = &mut function.body {
        visitor.statement(body)?;
    }
    Ok(())
}

pub fn walk_parameters(
    visitor: &mut impl VisitorMut,
    parameters: &mut [GenericParameter],
) -> Result<(), Diagnostic> {
    for parameter in parameters {
        visitor.span(&mut parameter.span);
        for constraint in &mut parameter.constraints {
            visitor.ty(constraint)?;
        }
        if let Some(default) = &mut parameter.default {
            visitor.ty(default)?;
        }
    }
    Ok(())
}
