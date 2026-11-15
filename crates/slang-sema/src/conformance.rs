//! Nominal interface inheritance and direct method requirement signatures.

pub(crate) mod witness;

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

/// Recover the interface origin of a specialized default body from its metadata.
pub(crate) fn default_requirement<'a>(
    module: &'a Module,
    function: &Function,
) -> Option<(&'a str, &'a Function)> {
    let instance = module.generic_instances.iter().find(|instance| {
        instance.type_name == function.name && instance.context == function.span.context
    })?;
    let origin = instance.arguments.get(1)?;

    module.declarations.iter().find_map(|declaration| {
        let Declaration::Interface(interface) = declaration else {
            return None;
        };

        if interface.name != origin.name {
            return None;
        }

        interface.requirements.iter().find_map(|requirement| {
            (requirement.body.is_some()
                && requirement.name == instance.template_name
                && requirement.span.source == function.span.source
                && requirement.span.start == function.span.start
                && requirement.span.end == function.span.end)
                .then_some((interface.name.as_str(), requirement))
        })
    })
}

impl Checker {
    pub(crate) fn declare_conformances(&mut self, module: &Module) {
        let mut inheritance = BTreeMap::new();

        for interface in self.interface_definitions.values().cloned().collect::<Vec<_>>() {
            self.current_module =
                self.symbols.get(&interface.name).map_or(Default::default(), |o| o.module);
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
                            && parameter.modifiers.iter().any(|m| m == "out" || m == "inout")
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

                if !requirement.generic_parameters.is_empty() {
                    self.error(
                        "T001",
                        requirement.span,
                        "generic interface requirements are not implemented",
                    );
                }

                if requirement.constructor.is_some() && requirement.body.is_some() {
                    self.error(
                        "T001",
                        requirement.span,
                        "interface constructor requirements cannot have a body",
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

        for interface in self.interface_definitions.values().cloned().collect::<Vec<_>>() {
            match interface_closure(&interface.name, &inheritance, interface.span) {
                Ok(bases) => {
                    self.conformances.insert(interface.name, bases);
                }

                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }

        for structure in self.struct_definitions.values().cloned().collect::<Vec<_>>() {
            self.current_module =
                self.symbols.get(&structure.name).map_or(Default::default(), |o| o.module);
            self.current_static_type = Some(structure.name.clone());
            let mut interfaces = BTreeSet::new();

            for base in self.interface_bases(&structure.bases) {
                if let Some(bases) = self.conformances.get(&base) {
                    interfaces.extend(bases.iter().cloned());
                }

                interfaces.insert(base);
            }

            self.conformances.insert(structure.name.clone(), interfaces.clone());
            let requirements = interfaces
                .iter()
                .flat_map(|name| {
                    self.interface_definitions[name]
                        .requirements
                        .iter()
                        .cloned()
                        .map(|f| (name.clone(), f))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if self.symbolic_types.contains(&structure.name) {
                self.declare_symbolic_requirements(&structure.name, &requirements);
            } else {
                self.declare_default_methods(module, &structure.name, &requirements);

                for (interface, requirement) in &requirements {
                    self.check_requirement(
                        module,
                        &structure.name,
                        interface,
                        requirement,
                        structure.span,
                    );
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

    pub(crate) fn requirement_signature(
        &mut self,
        owner: &str,
        requirement: &Function,
    ) -> (Function, Vec<Type>, Type) {
        let mut function = requirement.clone();
        function.body = None;
        RequirementOwner(owner)
            .function(&mut function)
            .expect("requirement substitution cannot fail");
        let parameters = function.parameters.iter().map(|p| self.resolve_type(&p.ty)).collect();
        let result = self.resolve_type(&function.return_type);
        (function, parameters, result)
    }

    fn declare_symbolic_requirements(&mut self, owner: &str, requirements: &[(String, Function)]) {
        for (index, (interface, requirement)) in requirements.iter().enumerate() {
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
                module: self
                    .symbols
                    .get(interface)
                    .map_or(self.current_module, |origin| origin.module),
                parameters,
                output_parameters: function
                    .parameters
                    .iter()
                    .map(|p| p.modifiers.iter().any(|m| m == "out" || m == "inout"))
                    .collect(),
                result,
                defined: true,
                public: function.modifiers.iter().any(|modifier| modifier == "public"),
                target_name: name.clone(),
                defaults: function.parameters.iter().map(|p| p.initializer.clone()).collect(),
                method: function.method.clone(),
                default_type_arguments: 0,
                generic: false,
            };

            self.symbolic_requirements.insert(
                (signature.declaration, owner.into()),
                ConstraintCallInfo {
                    owner: witness::reference(owner.into(), function.span),
                    interface: witness::reference(interface.clone(), function.span),
                    requirement: function,
                    declaration_module: self
                        .symbols
                        .get(interface)
                        .map_or(self.current_module.0, |o| o.module.0),
                },
            );
            self.direct_witnesses
                .insert(witness::key(owner, interface, requirement.span), signature.clone());

            if !self.requirement_shadowed(owner, interface, requirement, requirements) {
                self.functions.entry(name).or_default().push(signature);
            }
        }
    }

    /// A derived interface can replace one inherited overload without changing
    /// the witnesses retained for calls constrained to the base interface.
    fn requirement_shadowed(
        &mut self,
        owner: &str,
        interface: &str,
        requirement: &Function,
        requirements: &[(String, Function)],
    ) -> bool {
        let (_, parameters, _) = self.requirement_signature(owner, requirement);

        requirements.iter().any(|(other, candidate)| {
            if other == interface
                || !self.conformances.get(other).is_some_and(|bases| bases.contains(interface))
                || candidate.method.as_ref().map(|m| (&m.name, m.is_static))
                    != requirement.method.as_ref().map(|m| (&m.name, m.is_static))
                || candidate.constructor.is_some() != requirement.constructor.is_some()
            {
                return false;
            }

            self.requirement_signature(owner, candidate).1 == parameters
        })
    }

    pub(crate) fn default_signature(
        &self,
        module: &Module,
        owner: &str,
        interface: &str,
        requirement: &Function,
    ) -> Option<Signature> {
        module.declarations.iter().find_map(|declaration| {
            let Declaration::Function(function) = declaration else {
                return None;
            };

            if function.method.as_ref()?.owner != owner {
                return None;
            }

            let (origin, original) = default_requirement(module, function)?;

            if origin != interface || original.span != requirement.span {
                return None;
            }

            self.functions
                .get(&function.name)?
                .iter()
                .find(|signature| signature.declaration == span_key(function.span))
                .cloned()
        })
    }

    fn declare_default_methods(
        &mut self,
        module: &Module,
        owner: &str,
        requirements: &[(String, Function)],
    ) {
        let mut groups = BTreeMap::<String, Vec<Signature>>::new();

        for (interface, requirement) in requirements {
            let Some(method) = &requirement.method else {
                continue;
            };

            // Concrete methods hide inherited defaults by name. Constrained
            // calls still retain their separately recorded requirement witness.
            if self.methods.contains_key(&(owner.into(), method.name.clone()))
                || self.requirement_shadowed(owner, interface, requirement, requirements)
            {
                continue;
            }

            if let Some(signature) = self.default_signature(module, owner, interface, requirement) {
                groups.entry(method.name.clone()).or_default().push(signature);
            }
        }

        for (method, signatures) in groups {
            let mut name = format!("{owner}_default_lookup_{method}");

            while self.functions.contains_key(&name) {
                name.push('_');
            }

            self.methods.insert((owner.into(), method), name.clone());
            self.functions.insert(name, signatures);
        }
    }

    fn check_requirement(
        &mut self,
        module: &Module,
        owner: &str,
        interface: &str,
        requirement: &Function,
        span: Span,
    ) {
        let (required, parameters, result) = self.requirement_signature(owner, requirement);
        let name = required.method.as_ref().map_or("__init", |m| m.name.as_str());
        let target = if required.constructor.is_some() {
            self.constructors.get(owner)
        } else {
            self.methods.get(&(owner.into(), name.into()))
        };

        let overloads = target.and_then(|target| self.functions.get(target));
        let concrete_declaration = |signature: &Signature| {
            module.declarations.iter().find_map(|declaration| match declaration {
                Declaration::Function(function)
                    if span_key(function.span) == signature.declaration
                        && default_requirement(module, function).is_none() =>
                {
                    Some(function)
                }
                _ => None,
            })
        };
        let witness = overloads
            .and_then(|overloads| {
                overloads.iter().find(|signature| {
                    signature.defined
                        && signature.parameters == parameters
                        && signature.result == result
                        && signature.method.as_ref().map(|method| method.is_static)
                            == required.method.as_ref().map(|method| method.is_static)
                        && concrete_declaration(signature).is_some_and(|function| {
                            parameter_modes(function) == parameter_modes(&required)
                        })
                })
            })
            .cloned();

        if witness.is_none()
            && requirement.body.is_some()
            && overloads.is_some_and(|overloads| {
                overloads.iter().any(|signature| concrete_declaration(signature).is_some())
            })
        {
            self.error(
                "T032",
                span,
                "interface witness adaptation for a concrete same-name method is not implemented",
            );
            return;
        }

        let witness =
            witness.or_else(|| self.default_signature(module, owner, interface, requirement));

        if let Some(witness) = &witness {
            if requirement.body.is_some()
                && let Some(function) = module.declarations.iter().find_map(|d| match d {
                    Declaration::Function(f) if span_key(f.span) == witness.declaration => Some(f),
                    _ => None,
                })
                && !function.modifiers.iter().any(|m| m == "override")
            {
                self.error(
                    "T032",
                    function.span,
                    "missing 'override' keyword for an interface default implementation",
                );
            }

            self.interface_witnesses.insert(witness.declaration);
            self.direct_witnesses
                .insert(witness::key(owner, interface, requirement.span), witness.clone());
        }

        if witness.is_none()
            && !parameters.iter().chain(std::iter::once(&result)).any(|t| *t == Type::Error)
        {
            self.error(
                "T032",
                span,
                format!(
                    "type '{}' does not implement interface requirement '{name}' with the required signature",
                    self.source_name(owner),
                ),
            );
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
        | Type::Buffer {
            element,
            ..
        }
        | Type::ConstantBuffer {
            element,
            ..
        }
        | Type::Texture {
            element,
            ..
        } => contains_interface(element),
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
