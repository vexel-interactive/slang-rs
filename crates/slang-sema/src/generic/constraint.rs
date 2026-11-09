//! Constraint viability is nominal and uses the same canonical type instances.

use super::*;

pub(super) fn validate_parameters(parameters: &[GenericParameter]) -> Result<(), Diagnostic> {
    for (index, parameter) in parameters.iter().enumerate() {
        let forward = parameters[index + 1..]
            .iter()
            .map(|p| p.name.clone())
            .collect::<BTreeSet<_>>();
        for constraint in &parameter.constraints {
            if !argument_names(std::slice::from_ref(constraint)).is_disjoint(&forward) {
                return Err(Diagnostic::error(
                    "T032",
                    constraint.span,
                    "interface constraint refers to a generic parameter before its declaration",
                ));
            }
        }
    }
    Ok(())
}

impl Specializer {
    pub(super) fn constraints_satisfied(
        &mut self,
        parameters: &[GenericParameter],
        substitutions: &BTreeMap<String, TypeRef>,
    ) -> Result<bool, Diagnostic> {
        for parameter in parameters {
            for constraint in &parameter.constraints {
                let mut constraint = constraint.clone();
                ConstraintSubstitution(substitutions).ty(&mut constraint)?;
                self.canonical(&mut constraint, 0)?;
                if !constraint.arrays.is_empty()
                    || !self
                        .shapes
                        .interface_definitions
                        .contains_key(&constraint.name)
                {
                    return Err(Diagnostic::error(
                        "T032",
                        constraint.span,
                        "generic constraints require an interface type",
                    ));
                }
                let argument = &substitutions[&parameter.name];
                if !argument.arrays.is_empty()
                    || !self.conforms_to(
                        &argument.name,
                        &constraint.name,
                        &mut BTreeSet::new(),
                        constraint.span,
                    )?
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn conforms_to(
        &mut self,
        name: &str,
        interface: &str,
        active: &mut BTreeSet<String>,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        if name == interface {
            return Ok(true);
        }
        if active.len() >= 32 || !active.insert(name.into()) {
            return Err(Diagnostic::error(
                "T032",
                span,
                "cyclic or excessively nested interface inheritance",
            ));
        }
        let bases = if let Some(structure) = self.shapes.struct_definitions.get(name) {
            structure.bases.clone()
        } else if let Some(interface) = self.shapes.interface_definitions.get(name) {
            interface.bases.clone()
        } else {
            vec![]
        };
        let mut result = false;
        for mut base in bases {
            self.canonical(&mut base, 0)?;
            if self.conforms_to(&base.name, interface, active, span)? {
                result = true;
                break;
            }
        }
        active.remove(name);
        Ok(result)
    }
}

pub(super) struct ConstraintSubstitution<'a>(pub &'a BTreeMap<String, TypeRef>);
impl VisitorMut for ConstraintSubstitution<'_> {
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        substitute::replace_type(ty, self.0);
        visit::walk_type(self, ty)
    }
}
