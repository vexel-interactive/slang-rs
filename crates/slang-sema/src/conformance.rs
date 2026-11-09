//! Nominal interface inheritance and direct method requirement signatures.

use crate::{
    Type,
    checker::{Checker, Signature, span_key},
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{
    visit::{self, VisitorMut},
    *,
};
use std::collections::{BTreeMap, BTreeSet};

impl Checker {
    pub(crate) fn declare_conformances(&mut self, module: &Module) {
        let mut inheritance = BTreeMap::new();
        for interface in self
            .interface_definitions
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            self.current_module = self
                .symbols
                .get(&interface.name)
                .map_or(Default::default(), |o| o.module);
            self.current_static_type = Some(interface.name.clone());
            self.check_visibility_modifiers(&interface.modifiers, interface.span);
            let bases = self.interface_bases(&interface.bases);
            inheritance.insert(interface.name.clone(), bases);
            for requirement in &interface.requirements {
                let (_, parameters, result) =
                    self.requirement_signature(&interface.name, requirement);
                if interface.modifiers.iter().any(|m| m == "public") {
                    self.require_public_type(&result, requirement.span);
                    for (parameter, ty) in requirement.parameters.iter().zip(&parameters) {
                        self.require_public_type(ty, parameter.span);
                    }
                }
                for (parameter, ty) in requirement.parameters.iter().zip(&parameters) {
                    let directions = parameter
                        .modifiers
                        .iter()
                        .filter(|m| matches!(m.as_str(), "in" | "out" | "inout"))
                        .count();
                    if directions > 1
                        || parameter.modifiers.iter().any(|m| m == "const")
                            && parameter
                                .modifiers
                                .iter()
                                .any(|m| m == "out" || m == "inout")
                    {
                        self.error(
                            "T001",
                            parameter.span,
                            "conflicting interface parameter modifiers",
                        );
                    }
                    if *ty == Type::Void {
                        self.error(
                            "T002",
                            parameter.span,
                            "an interface parameter cannot have type void",
                        );
                    }
                }
                if !requirement.generic_parameters.is_empty() || requirement.body.is_some() {
                    self.error(
                        "T001",
                        requirement.span,
                        "generic requirements and default interface bodies are not implemented",
                    );
                }
                if requirement
                    .modifiers
                    .iter()
                    .any(|m| !matches!(m.as_str(), "public" | "internal" | "static"))
                    || requirement
                        .attributes
                        .iter()
                        .any(|a| a.name != "mutating" && a.name != "nonmutating")
                    || requirement.semantic.is_some()
                {
                    self.error(
                        "T001",
                        requirement.span,
                        "unsupported interface requirement modifier, attribute or semantic",
                    );
                }
            }
        }
        for interface in self
            .interface_definitions
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            match interface_closure(&interface.name, &inheritance, interface.span) {
                Ok(bases) => {
                    self.conformances.insert(interface.name, bases);
                }
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        for structure in self
            .struct_definitions
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            self.current_module = self
                .symbols
                .get(&structure.name)
                .map_or(Default::default(), |o| o.module);
            self.current_static_type = Some(structure.name.clone());
            let mut interfaces = BTreeSet::new();
            for base in self.interface_bases(&structure.bases) {
                if let Some(bases) = self.conformances.get(&base) {
                    interfaces.extend(bases.iter().cloned());
                }
                interfaces.insert(base);
            }
            self.conformances
                .insert(structure.name.clone(), interfaces.clone());
            let requirements = interfaces
                .iter()
                .flat_map(|name| self.interface_definitions[name].requirements.clone())
                .collect::<Vec<_>>();
            if self.symbolic_types.contains(&structure.name) {
                self.declare_symbolic_requirements(&structure.name, &requirements);
            } else {
                for requirement in &requirements {
                    self.check_requirement(module, &structure.name, requirement, structure.span);
                }
            }
        }
        self.current_module = Default::default();
        self.current_static_type = None;
    }

    fn interface_bases(&mut self, references: &[TypeRef]) -> BTreeSet<String> {
        let mut bases = BTreeSet::new();
        for reference in references {
            match self.resolve_type(reference) {
                Type::Interface(name) => {
                    if !bases.insert(name) {
                        self.error("T032", reference.span, "duplicate interface base");
                    }
                }
                Type::Error => {}
                _ => self.error(
                    "T032",
                    reference.span,
                    "inheritance and constraints require an interface type",
                ),
            }
        }
        bases
    }

    fn requirement_signature(
        &mut self,
        owner: &str,
        requirement: &Function,
    ) -> (Function, Vec<Type>, Type) {
        let mut function = requirement.clone();
        RequirementOwner(owner)
            .function(&mut function)
            .expect("requirement substitution cannot fail");
        let parameters = function
            .parameters
            .iter()
            .map(|p| self.resolve_type(&p.ty))
            .collect();
        let result = self.resolve_type(&function.return_type);
        (function, parameters, result)
    }

    fn declare_symbolic_requirements(&mut self, owner: &str, requirements: &[Function]) {
        for (index, requirement) in requirements.iter().enumerate() {
            let (function, parameters, result) = self.requirement_signature(owner, requirement);
            let name = if let Some(method) = &function.method {
                let key = (owner.into(), method.name.clone());
                self.methods
                    .entry(key)
                    .or_insert_with(|| format!("{owner}_requirement_{index}"))
                    .clone()
            } else {
                self.constructors
                    .entry(owner.into())
                    .or_insert_with(|| format!("{owner}_constructor_requirement"))
                    .clone()
            };
            let signature = Signature {
                declaration: span_key(function.span),
                parameters,
                output_parameters: function
                    .parameters
                    .iter()
                    .map(|p| p.modifiers.iter().any(|m| m == "out" || m == "inout"))
                    .collect(),
                result,
                defined: true,
                public: true,
                target_name: name.clone(),
                defaults: function
                    .parameters
                    .iter()
                    .map(|p| p.initializer.clone())
                    .collect(),
                method: function.method,
                default_type_arguments: 0,
                generic: false,
            };
            let overloads = self.functions.entry(name).or_default();
            if !overloads.iter().any(|s| {
                s.parameters == signature.parameters
                    && s.result == signature.result
                    && s.output_parameters == signature.output_parameters
                    && s.method.as_ref().map(|m| m.is_static)
                        == signature.method.as_ref().map(|m| m.is_static)
            }) {
                overloads.push(signature);
            }
        }
    }

    fn check_requirement(
        &mut self,
        module: &Module,
        owner: &str,
        requirement: &Function,
        span: Span,
    ) {
        let (required, parameters, result) = self.requirement_signature(owner, requirement);
        let name = required
            .method
            .as_ref()
            .map_or("__init", |m| m.name.as_str());
        let target = if required.constructor.is_some() {
            self.constructors.get(owner)
        } else {
            self.methods.get(&(owner.into(), name.into()))
        };
        let witness = target.and_then(|t| self.functions.get(t)).and_then(|overloads| overloads.iter().find(|signature| {
            signature.defined && signature.parameters == parameters && signature.result == result
                && signature.method.as_ref().map(|m| m.is_static) == required.method.as_ref().map(|m| m.is_static)
                && module.declarations.iter().any(|d| matches!(d, Declaration::Function(f) if span_key(f.span) == signature.declaration && parameter_modes(f) == parameter_modes(&required)))
        })).map(|signature| signature.declaration);
        if let Some(witness) = witness {
            self.interface_witnesses.insert(witness);
        }
        if witness.is_none()
            && !parameters
                .iter()
                .chain(std::iter::once(&result))
                .any(|t| *t == Type::Error)
        {
            self.error("T032", span, format!("type '{}' does not implement interface requirement '{name}' with the required signature", self.source_name(owner)));
        }
    }

    pub(crate) fn require_value_type(&mut self, ty: &Type, span: Span) {
        if contains_interface(ty) {
            self.error(
                "T001",
                span,
                "interface existential values and storage are not implemented",
            );
        }
    }

    pub(crate) fn constraint_witness_access(
        &self,
        signature: &Signature,
        owner: &str,
        span: Span,
    ) -> bool {
        self.interface_witnesses.contains(&signature.declaration)
            && self
                .generic_type_contexts
                .get(&span.context)
                .is_some_and(|types| types.contains(owner))
    }
}

pub(crate) fn interface_closure(
    root: &str,
    inheritance: &BTreeMap<String, BTreeSet<String>>,
    span: Span,
) -> Result<BTreeSet<String>, Diagnostic> {
    fn visit(
        root: &str,
        inheritance: &BTreeMap<String, BTreeSet<String>>,
        active: &mut BTreeSet<String>,
        output: &mut BTreeSet<String>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if active.len() >= 32 || !active.insert(root.into()) {
            return Err(Diagnostic::error(
                "T032",
                span,
                "cyclic or excessively nested interface inheritance",
            ));
        }
        if let Some(bases) = inheritance.get(root) {
            for base in bases {
                if !output.contains(base) {
                    visit(base, inheritance, active, output, span)?;
                }
            }
        }
        active.remove(root);
        output.insert(root.into());
        Ok(())
    }
    let mut output = BTreeSet::new();
    visit(root, inheritance, &mut BTreeSet::new(), &mut output, span)?;
    Ok(output)
}

fn parameter_modes(function: &Function) -> Vec<&str> {
    function
        .parameters
        .iter()
        .map(|p| {
            p.modifiers
                .iter()
                .find(|m| matches!(m.as_str(), "out" | "inout"))
                .map_or("in", String::as_str)
        })
        .collect()
}

fn contains_interface(ty: &Type) -> bool {
    match ty {
        Type::Interface(_) => true,
        Type::Array(element, _)
        | Type::Buffer { element, .. }
        | Type::ConstantBuffer { element, .. }
        | Type::Texture { element, .. } => contains_interface(element),
        _ => false,
    }
}

struct RequirementOwner<'a>(&'a str);
impl RequirementOwner<'_> {
    fn function(&mut self, function: &mut Function) -> Result<(), Diagnostic> {
        if let Some(owner) = &mut function.constructor {
            *owner = self.0.into();
        }
        if let Some(method) = &mut function.method {
            method.owner = self.0.into();
        }
        visit::walk_function(self, function)
    }
}
impl VisitorMut for RequirementOwner<'_> {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if ty.owner_ref || ty.name == "This" {
            ty.name = self.0.into();
            ty.base = None;
            ty.arguments.clear();
            ty.application = false;
        }
        visit::walk_type(self, ty)
    }
}
