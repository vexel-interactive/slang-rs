//! Bind a checked generic call to its conformance witness, retaining required defaults.

use crate::{
    Type,
    checker::{Binding, Checker, Signature, Storage},
};
use slang_module::ModuleId;
use slang_source::Span;
use slang_syntax::{ConstraintCallInfo, ExprId, TypeRef};
use std::collections::BTreeMap;

pub(crate) type WitnessKey = (String, String, usize, usize, usize);
pub(super) fn key(owner: &str, interface: &str, span: Span) -> WitnessKey {
    (
        owner.into(),
        interface.into(),
        span.source.0,
        span.start,
        span.end,
    )
}

pub(crate) fn reference(name: String, span: Span) -> TypeRef {
    TypeRef {
        name,
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

impl Checker {
    pub(crate) fn record_constraint_call(&mut self, callee: ExprId, signature: &Signature) {
        let owner = signature.method.as_ref().map(|m| m.owner.as_str()).or({
            if let Type::Struct(owner) = &signature.result {
                Some(owner.as_str())
            } else {
                None
            }
        });
        if let Some(owner) = owner
            && let Some(binding) = self
                .symbolic_requirements
                .get(&(signature.declaration, owner.into()))
        {
            self.constraint_calls.insert(callee, binding.clone());
        }
    }

    pub(crate) fn constraint_call_signature(
        &mut self,
        callee: ExprId,
        owner: &str,
    ) -> Option<Result<Signature, ()>> {
        let binding = self.constraint_calls.get(&callee)?.clone();
        let actual_owner = self.resolve_type(&binding.owner);
        let interface = self.resolve_type(&binding.interface);
        let Type::Interface(interface) = interface else {
            return Some(Err(()));
        };
        if actual_owner != Type::Struct(owner.into()) {
            self.error(
                "T032",
                binding.owner.span,
                "constrained call receiver changed during specialization",
            );
            return Some(Err(()));
        }
        let signature = self
            .direct_witnesses
            .get(&key(owner, &interface, binding.requirement.span))
            .cloned();
        let Some(mut signature) = signature else {
            self.error(
                "T032",
                binding.requirement.span,
                "selected interface requirement has no direct witness",
            );
            return Some(Err(()));
        };
        if binding
            .requirement
            .parameters
            .iter()
            .any(|p| p.initializer.is_some())
        {
            self.check_constraint_defaults(&binding);
        }
        signature.defaults = binding
            .requirement
            .parameters
            .iter()
            .map(|p| p.initializer.clone())
            .collect();
        Some(Ok(signature))
    }

    pub(crate) fn check_constraint_defaults(&mut self, binding: &ConstraintCallInfo) {
        let module = self.current_module;
        let current_type = self.current_type.take();
        let static_type = self.current_static_type.take();
        let scopes = self.scopes.split_off(1);
        self.current_module = ModuleId(binding.declaration_module);
        self.current_type = binding.requirement.method.as_ref().map(|m| m.owner.clone());
        self.current_static_type = Some(binding.owner.name.clone());
        let mut parameters = BTreeMap::new();
        for parameter in &binding.requirement.parameters {
            let ty = self.resolve_type(&parameter.ty);
            parameters.insert(
                parameter.name.clone(),
                Binding {
                    ty,
                    writable: !parameter.modifiers.iter().any(|m| m == "const"),
                    constant: None,
                    shape_constant: None,
                    storage: Storage::Local,
                },
            );
        }
        self.scopes.push(parameters);
        self.parameter_defaults(&binding.requirement);
        self.scopes.pop();
        self.scopes.extend(scopes);
        self.current_module = module;
        self.current_type = current_type;
        self.current_static_type = static_type;
    }
}
