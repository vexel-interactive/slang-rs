//! Ordinary lookup exposes visible interface operations through their witnesses.

use super::witness;
use crate::checker::{Checker, Signature};
use slang_source::{Diagnostic, Span};
use slang_syntax::{
    ConstraintCallInfo, TypeRef,
    visit::{self, VisitorMut},
};

impl Checker {
    pub(crate) fn projected_interface_methods(
        &mut self,
        owner: &str,
        name: &str,
        visible_methods: &[Signature],
        type_receiver: bool,
        span: Span,
    ) -> Vec<(Signature, ConstraintCallInfo)> {
        if self.symbolic_types.contains(owner)
            || self.interface_adapter_contexts.contains(&span.context)
        {
            return vec![];
        }

        let requirements = self
            .conformances
            .get(owner)
            .into_iter()
            .flatten()
            .flat_map(|interface| {
                let module = self.symbols.get(interface).map_or(Default::default(), |o| o.module);
                if module != self.current_module && !self.public_types.contains(interface) {
                    return vec![];
                }
                self.interface_definitions[interface]
                    .requirements
                    .iter()
                    .filter(|requirement| {
                        requirement.modifiers.iter().any(|m| m == "public")
                            || module == self.current_module
                    })
                    .cloned()
                    .map(|mut requirement| {
                        requirement.body = None;
                        visit::walk_function(
                            &mut OwnerContext {
                                owner,
                                context: span.context,
                            },
                            &mut requirement,
                        )
                        .expect("projection context assignment cannot fail");
                        (interface.clone(), requirement)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut projected = vec![];

        for (interface, requirement) in &requirements {
            let Some(method) = &requirement.method else {
                continue;
            };
            if method.name != name
                || type_receiver && !method.is_static
                || self.requirement_shadowed(owner, interface, requirement, &requirements)
            {
                continue;
            }

            let (required, parameters, result) = self.requirement_signature(owner, requirement);
            // Concrete methods hide the same inherited signature only where
            // that concrete declaration is accessible. Other overloads coexist.
            if visible_methods.iter().any(|signature| {
                signature.parameters.starts_with(&parameters)
                    && signature.defaults[parameters.len()..].iter().all(Option::is_some)
                    && signature.method.as_ref().is_some_and(|m| m.is_static == method.is_static)
            }) {
                continue;
            }

            let Some(mut signature) = self
                .direct_witnesses
                .get(&witness::key(owner, interface, requirement.span))
                .cloned()
            else {
                continue;
            };
            signature.parameters = parameters;
            signature.result = result;
            signature.output_parameters = required
                .parameters
                .iter()
                .map(|p| p.modifiers.iter().any(|m| m == "out" || m == "inout"))
                .collect();
            signature.defaults =
                required.parameters.iter().map(|p| p.initializer.clone()).collect();
            signature.method = required.method.clone();
            let declaration_module =
                self.symbols.get(interface).map_or(self.current_module.0, |o| o.module.0);
            projected.push((
                signature,
                ConstraintCallInfo {
                    owner: witness::reference(owner.into(), requirement.span),
                    interface: witness::reference(interface.clone(), requirement.span),
                    requirement: required,
                    declaration_module,
                },
            ));
        }

        projected
    }
}

struct OwnerContext<'a> {
    owner: &'a str,
    context: usize,
}

impl VisitorMut for OwnerContext<'_> {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        if ty.owner_ref || ty.name == "This" || ty.name == self.owner {
            ty.span = ty.span.with_context(self.context);
        }
        visit::walk_type(self, ty)
    }
}
