use crate::link::{Members, Names};
use crate::{ModuleId, SymbolOrigin};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Declaration, Expr, ExprId, ExprKind, Stmt, StmtKind, TypeRef, Variable};
use std::collections::BTreeMap;

/// Rewrite module symbols while preserving parameter and block-local bindings.
pub(crate) struct Resolver<'a> {
    own: &'a Names,
    imported: &'a Names,
    scopes: Vec<BTreeMap<String, String>>,
    next_expression: &'a mut usize,
    local_prefix: String,
    next_local: usize,
    module: ModuleId,
    origins: &'a mut BTreeMap<String, SymbolOrigin>,
    members: &'a Members,
    active_members: Option<std::collections::BTreeSet<String>>,
    active_owner: Option<String>,
}

impl<'a> Resolver<'a> {
    pub fn new(
        own: &'a Names,
        imported: &'a Names,
        next_expression: &'a mut usize,
        local_prefix: String,
        module: ModuleId,
        origins: &'a mut BTreeMap<String, SymbolOrigin>,
        members: &'a Members,
    ) -> Self {
        Self {
            own,
            imported,
            scopes: vec![],
            next_expression,
            local_prefix,
            next_local: 0,
            module,
            origins,
            members,
            active_members: None,
            active_owner: None,
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

    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        ty.name = self.symbol(&ty.name, true, ty.span)?;
        for argument in &mut ty.arguments {
            match argument {
                slang_syntax::TypeArgument::Type(argument) => self.ty(argument)?,
                slang_syntax::TypeArgument::Expression(argument) => self.expression(argument)?,
                slang_syntax::TypeArgument::Integer { .. } => {}
            }
        }
        for dimension in &mut ty.arrays {
            self.expression(dimension)?;
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

    /// A local initializer sees the previous binding before its new name is installed.
    fn local(&mut self, variable: &mut Variable) -> Result<(), Diagnostic> {
        self.variable(variable)?;
        self.bind_local(variable);
        Ok(())
    }

    fn bind_local(&mut self, variable: &mut Variable) {
        let source = variable.name.clone();
        if let Some(target) = self.scopes.last().unwrap().get(&source) {
            // Keep duplicate declarations mapped alike so semantic checking rejects them.
            variable.name = target.clone();
            return;
        }
        let outer = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&source));
        let captures = outer.is_some_and(|target| target == &source)
            || [&self.own.values, &self.imported.values]
                .iter()
                .any(|names| {
                    names
                        .get(&source)
                        .is_some_and(|targets| targets.contains(&source))
                });
        if captures {
            variable.name = format!("{}{}", self.local_prefix, self.next_local);
            self.next_local += 1;
            self.origins.insert(
                variable.name.clone(),
                SymbolOrigin {
                    module: self.module,
                    name: source.clone(),
                },
            );
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(source, variable.name.clone());
    }

    pub fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        match declaration {
            Declaration::TypeAlias(alias) => {
                alias.name = self.symbol(&alias.name, true, alias.span)?;
                self.ty(&mut alias.target)?;
            }
            Declaration::Struct(structure) => {
                self.active_owner = Some(structure.name.clone());
                self.active_members = self.members.static_names.get(&structure.name).cloned();
                structure.name = self.symbol(&structure.name, true, structure.span)?;
                for field in &mut structure.fields {
                    self.variable(field)?;
                }
                self.active_owner = None;
                self.active_members = None;
            }
            Declaration::Global(variable) => {
                self.active_owner = self.members.owners.get(&variable.name).cloned();
                self.active_members = self
                    .active_owner
                    .as_ref()
                    .and_then(|o| self.members.all.get(o))
                    .cloned();
                self.variable(variable)?;
                variable.name = self.symbol(&variable.name, false, variable.span)?;
                self.active_owner = None;
                self.active_members = None;
            }
            Declaration::Function(function) => {
                let members = function
                    .owner_type()
                    .and_then(|name| self.members.all.get(name))
                    .cloned();
                self.active_owner = function.owner_type().map(str::to_owned);
                self.active_members = self
                    .active_owner
                    .as_ref()
                    .and_then(|o| self.members.static_names.get(o))
                    .cloned();
                if let Some(owner) = &mut function.constructor {
                    *owner = self.symbol(owner, true, function.span)?;
                }
                if let Some(method) = &mut function.method {
                    method.owner = self.symbol(&method.owner, true, function.span)?;
                }
                function.name = self.symbol(&function.name, false, function.span)?;
                self.ty(&mut function.return_type)?;
                for attribute in &mut function.attributes {
                    for argument in &mut attribute.arguments {
                        self.expression(argument)?;
                    }
                }
                self.scopes.push(BTreeMap::new());
                for parameter in &mut function.parameters {
                    if parameter.name == "this"
                        && function.method.as_ref().is_some_and(|m| !m.is_static)
                    {
                        self.scopes
                            .last_mut()
                            .unwrap()
                            .insert("this".into(), "this".into());
                        continue;
                    }
                    self.bind_local(parameter);
                }
                if function.method.is_some() {
                    self.active_members = members.clone();
                }
                for parameter in &mut function.parameters {
                    self.variable(parameter)?;
                }
                if function.constructor.is_some() {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert("this".into(), "this".into());
                }
                self.active_members = members;
                if let Some(body) = &mut function.body {
                    self.statement(body)?;
                }
                self.scopes.pop();
                self.active_members = None;
                self.active_owner = None;
            }
        }
        Ok(())
    }

    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(statements) => {
                self.scopes.push(BTreeMap::new());
                for statement in statements {
                    self.statement(statement)?;
                }
                self.scopes.pop();
            }
            StmtKind::Variable(variable) => self.local(variable)?,
            StmtKind::Variables(variables) => {
                for variable in variables {
                    self.local(variable)?;
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
                self.statement(then_branch)?;
                if let Some(branch) = else_branch {
                    self.statement(branch)?;
                }
            }
            StmtKind::While { condition, body } => {
                self.expression(condition)?;
                self.statement(body)?;
            }
            StmtKind::DoWhile { condition, body } => {
                self.statement(body)?;
                self.expression(condition)?;
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(BTreeMap::new());
                self.statement(initializer)?;
                if let Some(condition) = condition {
                    self.expression(condition)?;
                }
                if let Some(increment) = increment {
                    self.expression(increment)?;
                }
                self.statement(body)?;
                self.scopes.pop();
            }
            StmtKind::Switch { selector, cases } => {
                self.expression(selector)?;
                self.scopes.push(BTreeMap::new());
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
            StmtKind::Break
            | StmtKind::Continue
            | StmtKind::Discard
            | StmtKind::Return(None)
            | StmtKind::Empty => {}
        }
        Ok(())
    }

    pub fn static_member(
        &self,
        member: &mut slang_syntax::StaticMemberInfo,
    ) -> Result<(), Diagnostic> {
        member.owner = self.symbol(&member.owner, true, member.span)?;
        member.variable_name = self.symbol(&member.variable_name, false, member.span)?;
        Ok(())
    }

    pub fn constant_buffer(
        &self,
        buffer: &mut slang_syntax::ConstantBufferInfo,
    ) -> Result<(), Diagnostic> {
        buffer.variable_name = self.symbol(&buffer.variable_name, false, buffer.span)?;
        buffer.element_type = self.symbol(&buffer.element_type, true, buffer.span)?;
        for member in &mut buffer.members {
            member.variable_name = self.symbol(&member.variable_name, false, member.span)?;
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
                if let Some(target) = self.scopes.iter().rev().find_map(|scope| scope.get(name)) {
                    *name = target.clone();
                } else if let Some(target) = self
                    .active_owner
                    .as_ref()
                    .and_then(|o| self.members.fields.get(&(o.clone(), name.clone())))
                {
                    *name = self.symbol(target, false, expression.span)?;
                } else if self
                    .active_members
                    .as_ref()
                    .is_some_and(|members| members.contains(name))
                {
                    // Members of the containing type win over module globals.
                } else {
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
            ExprKind::TypeApplication(target) => self.ty(target)?,
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
