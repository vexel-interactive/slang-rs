//! Preserve selections made while checking opaque type parameters.

use super::*;

pub(super) fn reference(name: &str, span: Span) -> TypeRef {
    TypeRef {
        name: name.into(),
        base: None,
        application: false,
        owner_ref: false,
        parameter_ref: false,
        arguments: vec![],
        arrays: vec![],
        matrix_major: None,
        span,
    }
}

pub(super) fn set_owner(binding: &mut ConstraintCallInfo) {
    if let Some(method) = &mut binding.requirement.method {
        method.owner = binding.owner.name.clone();
    }
    if let Some(owner) = &mut binding.requirement.constructor {
        *owner = binding.owner.name.clone();
    }
}

pub(super) fn template_calls(
    module: &Module,
    origins: &BTreeMap<ExprId, ExprId>,
    parameters: &BTreeMap<String, TypeRef>,
) -> Result<TemplateBindings, Diagnostic> {
    let mut calls = BTreeMap::new();
    let mut visitor = Reify {
        module,
        parameters,
        depth: 0,
    };
    for (id, binding) in &module.constraint_calls {
        let mut binding = binding.clone();
        visitor.ty(&mut binding.owner)?;
        visitor.ty(&mut binding.interface)?;
        visit::walk_function(&mut visitor, &mut binding.requirement)?;
        set_owner(&mut binding);
        calls.insert(origins.get(id).copied().unwrap_or(*id), binding);
    }
    let mut variables = vec![];
    for variable in &module.constraint_variables {
        let mut binding = variable.call.clone();
        visitor.ty(&mut binding.owner)?;
        visitor.ty(&mut binding.interface)?;
        visit::walk_function(&mut visitor, &mut binding.requirement)?;
        set_owner(&mut binding);
        variables.push(ConstraintVariableInfo {
            span: variable.span.with_context(0),
            call: binding,
        });
    }
    Ok(TemplateBindings { calls, variables })
}

struct Reify<'a> {
    module: &'a Module,
    parameters: &'a BTreeMap<String, TypeRef>,
    depth: usize,
}

impl Reify<'_> {
    fn named(&mut self, name: &str, span: Span) -> Result<TypeRef, Diagnostic> {
        if self.depth >= 64 {
            return Err(error(span, "constraint type reconstruction limit exceeded"));
        }
        self.depth += 1;
        let result = self.named_inner(name, span);
        self.depth -= 1;
        result
    }

    fn named_inner(&mut self, name: &str, span: Span) -> Result<TypeRef, Diagnostic> {
        if let Some(parameter) = self.parameters.get(name) {
            return Ok(parameter.clone());
        }
        if let Some(instance) = self
            .module
            .generic_instances
            .iter()
            .find(|i| i.type_name == name)
        {
            let mut ty = self.named(&instance.template_name, span)?;
            ty.application = true;
            ty.arguments = instance.arguments[..instance.parameter_count]
                .iter()
                .cloned()
                .map(TypeArgument::Type)
                .collect();
            for argument in &mut ty.arguments {
                if let TypeArgument::Type(argument) = argument {
                    self.ty(argument)?;
                }
            }
            return Ok(ty);
        }
        if let Some(nested) = self
            .module
            .nested_types
            .iter()
            .find(|n| n.type_name == name)
        {
            let mut ty = reference(&nested.name, span);
            ty.base = Some(Box::new(self.named(&nested.owner, span)?));
            return Ok(ty);
        }
        Ok(reference(name, span))
    }
}

impl VisitorMut for Reify<'_> {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::Name(name) = &expression.kind {
            let instance = self
                .module
                .generic_instances
                .iter()
                .find(|i| i.type_name == *name)
                .or_else(|| {
                    self.module
                        .function_applications
                        .iter()
                        .find(|a| a.name == *name)
                        .and_then(|a| {
                            a.candidates.iter().find_map(|c| {
                                self.module
                                    .generic_instances
                                    .iter()
                                    .find(|i| i.type_name == c.function)
                            })
                        })
                });
            if let Some(instance) = instance {
                let mut ty = self.named(&instance.template_name, expression.span)?;
                ty.application = true;
                ty.arguments = instance.arguments[..instance.parameter_count]
                    .iter()
                    .cloned()
                    .map(TypeArgument::Type)
                    .collect();
                expression.kind = ExprKind::TypeApplication(Box::new(ty));
            }
        }
        visit::walk_expression(self, expression)
    }
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if ty.base.is_none() {
            let mut named = self.named(&ty.name, ty.span)?;
            if named.name != ty.name || named.base.is_some() || named.application {
                named.arrays = std::mem::take(&mut ty.arrays);
                named.matrix_major = ty.matrix_major;
                named.parameter_ref = ty.parameter_ref;
                named.span = ty.span;
                if ty.application {
                    named.application = true;
                    named.arguments = std::mem::take(&mut ty.arguments);
                }
                *ty = named;
            }
        }
        visit::walk_type(self, ty)
    }
}

impl Specializer {
    pub(super) fn specialize_constraint_variable(&mut self, span: Span) -> Result<(), Diagnostic> {
        let site = crate::span_key(span);
        if !self.constraint_variables.contains_key(&site)
            || !self.processed_constraint_variables.insert(site)
        {
            return Ok(());
        }
        let mut binding = self.constraint_variables.remove(&site).unwrap();
        self.specialize_constraint_binding(&mut binding)?;
        self.constraint_variables.insert(site, binding);
        Ok(())
    }
    pub(super) fn specialize_constraint_call(&mut self, id: ExprId) -> Result<(), Diagnostic> {
        if !self.constraint_calls.contains_key(&id) || !self.processed_constraint_calls.insert(id) {
            return Ok(());
        }
        let mut binding = self.constraint_calls.remove(&id).unwrap();
        self.specialize_constraint_binding(&mut binding)?;
        self.constraint_calls.insert(id, binding);
        Ok(())
    }

    fn specialize_constraint_binding(
        &mut self,
        binding: &mut ConstraintCallInfo,
    ) -> Result<(), Diagnostic> {
        self.ty(&mut binding.owner)?;
        self.ty(&mut binding.interface)?;
        let module = self.shapes.current_module;
        let owner = self.shapes.current_static_type.take();
        let scopes = self.shapes.scopes.split_off(1);
        self.shapes.current_module = slang_module::ModuleId(binding.declaration_module);
        self.shapes.current_static_type = Some(binding.owner.name.clone());
        self.shapes.scopes.push(BTreeMap::new());
        visit::walk_function(self, &mut binding.requirement)?;
        self.shapes.scopes.pop();
        self.shapes.scopes.extend(scopes);
        self.shapes.current_module = module;
        self.shapes.current_static_type = owner;
        set_owner(binding);
        Ok(())
    }
}
