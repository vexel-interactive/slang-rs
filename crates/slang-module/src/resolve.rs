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
    generic_types: BTreeMap<String, String>,
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
            generic_types: BTreeMap::new(),
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
        if let Some(base) = &mut ty.base {
            self.ty(base)?;
            return self.type_arguments(ty);
        }
        let mut parts = ty.name.split("::");
        let base = parts.next().unwrap();
        let qualified = ty.name.contains("::");
        let local = qualified
            .then(|| self.scopes.iter().rev().find_map(|scope| scope.get(base)))
            .flatten();
        let mut resolved = if let Some(parameter) = self.generic_types.get(base) {
            parameter.clone()
        } else if let Some(local) = local {
            local.clone()
        } else {
            let member = self.member_scope(base).is_some();
            let source = base;
            let base = self
                .member_target(base, true)
                .or_else(|| qualified.then(|| self.member_target(base, false)).flatten())
                .unwrap_or(base);
            if member && base == source {
                base.to_owned()
            } else {
                let target = self.symbol(base, true, ty.span)?;
                if qualified && target == base {
                    self.symbol(base, false, ty.span)?
                } else {
                    target
                }
            }
        };
        for member in parts {
            resolved.push_str("::");
            resolved.push_str(member);
        }
        ty.name = resolved;
        self.type_arguments(ty)
    }

    fn type_arguments(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
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
        let captures = self.generic_types.contains_key(&source)
            || outer.is_some_and(|target| target == &source)
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
                self.active_owner = self.members.type_owners.get(&alias.name).cloned();
                self.generic_types = self.generic_bindings(Some(&alias.name));
                self.generic_parameters(&mut alias.generic_parameters)?;
                self.active_members = self.owner_members(false);
                alias.name = self.symbol(&alias.name, true, alias.span)?;
                self.ty(&mut alias.target)?;
                self.active_owner = None;
                self.active_members = None;
                self.generic_types.clear();
            }
            Declaration::Struct(structure) => {
                self.active_owner = Some(structure.name.clone());
                self.generic_types = self.generic_bindings(Some(&structure.name));
                self.generic_parameters(&mut structure.generic_parameters)?;
                self.active_members = self.owner_members(false);
                structure.name = self.symbol(&structure.name, true, structure.span)?;
                for field in &mut structure.fields {
                    self.variable(field)?;
                }
                self.active_owner = None;
                self.active_members = None;
                self.generic_types.clear();
            }
            Declaration::Global(variable) => {
                self.active_owner = self.members.owners.get(&variable.name).cloned();
                self.generic_types = self.generic_bindings(self.active_owner.as_deref());
                self.active_members = self.owner_members(true);
                self.variable(variable)?;
                variable.name = self.symbol(&variable.name, false, variable.span)?;
                self.active_owner = None;
                self.active_members = None;
                self.generic_types.clear();
            }
            Declaration::Function(function) => {
                self.active_owner = function.owner_type().map(str::to_owned);
                self.generic_types = self.generic_bindings(self.active_owner.as_deref());
                for (index, parameter) in function.generic_parameters.iter().enumerate() {
                    self.generic_types.insert(
                        parameter.name.clone(),
                        format!(
                            "{}generic_function_{}_{}_{index}",
                            self.local_prefix,
                            function.name.len(),
                            function.name
                        ),
                    );
                }
                self.generic_parameters(&mut function.generic_parameters)?;
                let members = self.owner_members(true);
                self.active_members = self.owner_members(false);
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
                self.generic_types.clear();
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

    fn generic_bindings(&self, owner: Option<&str>) -> BTreeMap<String, String> {
        let mut owner = owner.map(str::to_owned);
        let mut parameters = BTreeMap::new();
        while let Some(current) = owner {
            if let Some(names) = self.members.generic_parameters.get(&current) {
                for name in names {
                    parameters.entry(name.clone()).or_insert_with(|| {
                        format!(
                            "{}generic_{}_{}_{name}",
                            self.local_prefix,
                            current.len(),
                            current
                        )
                    });
                }
            }
            owner = self.members.type_owners.get(&current).cloned();
        }
        parameters
    }

    fn generic_parameters(
        &mut self,
        parameters: &mut [slang_syntax::GenericParameter],
    ) -> Result<(), Diagnostic> {
        for parameter in parameters {
            parameter.name = self.generic_types[&parameter.name].clone();
            if let Some(default) = &mut parameter.default {
                self.ty(default)?;
            }
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

    pub fn nested_type(
        &mut self,
        member: &mut slang_syntax::NestedTypeInfo,
    ) -> Result<(), Diagnostic> {
        member.owner = self.symbol(&member.owner, true, member.span)?;
        member.type_name = self.symbol(&member.type_name, true, member.span)?;
        let owner = self
            .origins
            .get(&member.owner)
            .map_or(member.owner.as_str(), |o| &o.name);
        let name = format!("{owner}.{}", member.name);
        if let Some(origin) = self.origins.get_mut(&member.type_name) {
            origin.name = name;
        }
        Ok(())
    }

    fn member_target<'b>(&'b self, name: &str, is_type: bool) -> Option<&'b str> {
        let owner = self.member_scope(name)?;
        let members = if is_type {
            &self.members.types
        } else {
            &self.members.fields
        };
        members
            .get(&(owner.into(), name.into()))
            .map(String::as_str)
    }

    fn member_scope(&self, name: &str) -> Option<&str> {
        let mut owner = self.active_owner.as_ref();
        while let Some(current) = owner {
            if self
                .members
                .all
                .get(current)
                .is_some_and(|m| m.contains(name))
            {
                return Some(current);
            }
            owner = self.members.type_owners.get(current);
        }
        None
    }

    fn owner_members(&self, instance: bool) -> Option<std::collections::BTreeSet<String>> {
        let mut names = std::collections::BTreeSet::new();
        let mut owner = self.active_owner.as_ref();
        while let Some(current) = owner {
            let members = if instance {
                &self.members.all
            } else {
                &self.members.static_names
            };
            if let Some(members) = members.get(current) {
                names.extend(members.iter().cloned());
            }
            owner = self.members.type_owners.get(current);
        }
        self.active_owner.as_ref().map(|_| names)
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
        self.comparison_application(expression);
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
                } else if let Some(target) = self.generic_types.get(name) {
                    *name = target.clone();
                } else if let Some(target) = self.member_target(name, false) {
                    *name = self.symbol(target, false, expression.span)?;
                } else if let Some(target) = self.member_target(name, true) {
                    *name = self.symbol(target, true, expression.span)?;
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
            ExprKind::TypeApplication(target) => {
                let local = self
                    .scopes
                    .iter()
                    .rev()
                    .any(|s| s.contains_key(&target.name));
                let generic = self.own.generic_functions.contains(&target.name)
                    || !self.own.values.contains_key(&target.name)
                        && self.imported.generic_functions.contains(&target.name);
                if target.base.is_none()
                    && generic
                    && !local
                    && !self.generic_types.contains_key(&target.name)
                {
                    target.name = self.symbol(&target.name, false, target.span)?;
                    self.type_arguments(target)?;
                } else {
                    self.ty(target)?;
                }
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

    /// A local value followed by `< b > (c)` is a pair of comparisons.
    fn comparison_application(&self, expression: &mut Expr) {
        let ExprKind::Call { callee, arguments } = &expression.kind else {
            return;
        };
        let ExprKind::TypeApplication(ty) = &callee.kind else {
            return;
        };
        if arguments.len() != 1 || ty.base.is_some() || !ty.arrays.is_empty() {
            return;
        }
        if !self.scopes.iter().rev().any(|s| s.contains_key(&ty.name))
            && (self.own.generic_functions.contains(&ty.name)
                || !self.own.values.contains_key(&ty.name)
                    && self.imported.generic_functions.contains(&ty.name))
        {
            return;
        }
        let value = self.scopes.iter().rev().any(|s| s.contains_key(&ty.name))
            || self.own.values.contains_key(&ty.name) && !self.own.types.contains_key(&ty.name)
            || self.imported.values.contains_key(&ty.name)
                && !self.imported.types.contains_key(&ty.name);
        if !value {
            return;
        }
        let [slang_syntax::TypeArgument::Type(right)] = ty.arguments.as_slice() else {
            return;
        };
        if right.application
            || !right.arrays.is_empty()
            || !right.arguments.is_empty()
            || right.base.is_some()
        {
            return;
        }
        let name = |name: String, span| {
            let mut value = *callee.clone();
            value.kind = ExprKind::Name(name);
            value.span = span;
            value.parenthesized = false;
            value
        };
        let mut parts = right.name.split("::");
        let mut right_value = name(parts.next().unwrap().into(), right.span);
        for member in parts {
            let mut value = *callee.clone();
            value.kind = ExprKind::Member {
                base: Box::new(right_value),
                name: member.into(),
                static_lookup: false,
            };
            right_value = value;
        }
        let mut comparison = *callee.clone();
        comparison.kind = ExprKind::Binary {
            op: "<".into(),
            left: Box::new(name(ty.name.clone(), ty.span)),
            right: Box::new(right_value),
        };
        expression.kind = ExprKind::Binary {
            op: ">".into(),
            left: Box::new(comparison),
            right: Box::new(arguments[0].clone()),
        };
    }
}
