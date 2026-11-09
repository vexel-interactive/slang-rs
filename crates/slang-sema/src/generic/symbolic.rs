//! Opaque parameter instances checked by the ordinary semantic pipeline.

use super::*;
use crate::{Type, checker::Checker};

impl Specializer {
    pub(super) fn symbolic_instance(&mut self, name: &str) -> Result<(), Diagnostic> {
        let template = self.templates[name].clone();
        let module = self
            .symbols
            .get(&template.name)
            .map_or(Default::default(), |o| o.module);
        let mut arguments = vec![];
        let mut declarations = vec![];
        for parameter in &template.parameters {
            let context = self.next_context;
            self.next_context += 1;
            let name = format!("{}parameter_{context}", self.prefix);
            let span = parameter.span.with_context(context);
            self.symbolic_types.insert(name.clone());
            self.symbolic_parameters.insert(
                name.clone(),
                witness::reference(&parameter.name, parameter.span),
            );
            self.symbols.insert(
                name.clone(),
                SymbolOrigin {
                    module,
                    name: "generic type parameter".into(),
                },
            );
            declarations.push(Declaration::Struct(Struct {
                name: name.clone(),
                generic_parameters: vec![],
                bases: parameter.constraints.clone(),
                fields: vec![],
                modifiers: vec!["public".into()],
                span,
            }));
            arguments.push(TypeArgument::Type(TypeRef {
                name,
                base: None,
                application: false,
                owner_ref: false,
                parameter_ref: false,
                arguments: vec![],
                matrix_major: None,
                arrays: vec![],
                span,
            }));
        }
        let substitutions = template
            .parameters
            .iter()
            .zip(&arguments)
            .map(|(p, a)| {
                let TypeArgument::Type(ty) = a else {
                    unreachable!()
                };
                (p.name.clone(), ty.clone())
            })
            .collect::<BTreeMap<_, _>>();
        for declaration in &mut declarations {
            constraint::ConstraintSubstitution(&substitutions).declaration(declaration)?;
        }
        self.register(declarations, vec![], vec![], None)?;
        let target = self.instantiate(name, &arguments, template.parameters[0].span)?;
        let context = self
            .instance_output
            .iter()
            .find(|i| i.type_name == target)
            .unwrap()
            .context;
        self.validation_contexts.insert(context);
        Ok(())
    }
}

impl Checker {
    /// Symbolic numeric shapes have opaque components and no arithmetic witnesses.
    pub(crate) fn symbolic_shape(
        &mut self,
        component: Type,
        dimensions: Vec<u8>,
        span: Span,
    ) -> Type {
        if let Some((name, _)) = self
            .symbolic_shapes
            .iter()
            .find(|(_, shape)| shape.0 == component && shape.1 == dimensions)
        {
            return Type::Struct(name.clone());
        }
        let fields: BTreeMap<String, Type> = if dimensions.len() == 1 {
            ["x", "y", "z", "w"]
                .iter()
                .take(usize::from(dimensions[0]))
                .map(|n| ((*n).into(), component.clone()))
                .collect()
        } else {
            let row = self.symbolic_shape(component.clone(), vec![dimensions[1]], span);
            (0..dimensions[0])
                .map(|i| (format!("_row_{i}"), row.clone()))
                .collect()
        };
        let mut name = format!("_slang_symbolic_shape_{}", self.symbolic_shapes.len());
        while self.structures.contains_key(&name) {
            name.push('_');
        }
        let variables = fields
            .iter()
            .map(|(name, ty)| Variable {
                name: name.clone(),
                ty: TypeRef {
                    name: ty.to_string(),
                    base: None,
                    application: false,
                    owner_ref: false,
                    parameter_ref: false,
                    arguments: vec![],
                    matrix_major: None,
                    arrays: vec![],
                    span,
                },
                modifiers: vec!["public".into()],
                initializer: None,
                semantic: None,
                register: None,
                pack_offset: None,
                span,
            })
            .collect();
        self.structures.insert(name.clone(), fields);
        self.struct_definitions.insert(
            name.clone(),
            Struct {
                name: name.clone(),
                fields: variables,
                generic_parameters: vec![],
                bases: vec![],
                modifiers: vec!["public".into()],
                span,
            },
        );
        self.public_types.insert(name.clone());
        self.symbolic_shapes
            .insert(name.clone(), (component, dimensions));
        Type::Struct(name)
    }

    pub(crate) fn symbolic_selection(&mut self, owner: &str, member: &str, span: Span) -> Type {
        let (component, dimensions) = self.symbolic_shapes[owner].clone();
        let indices = if dimensions.len() == 1 {
            crate::swizzle::vector(member).filter(|v| v.iter().all(|&i| i < dimensions[0]))
        } else {
            crate::swizzle::matrix(member, dimensions[0], dimensions[1])
        };
        match indices {
            Some(indices) if indices.len() == 1 => component,
            Some(indices) => self.symbolic_shape(component, vec![indices.len() as u8], span),
            None => {
                self.error("T013", span, "invalid symbolic numeric selector");
                Type::Error
            }
        }
    }

    pub(crate) fn symbolic_index(&mut self, owner: &str, span: Span) -> (Type, Option<u32>) {
        let (component, dimensions) = self.symbolic_shapes[owner].clone();
        if dimensions.len() == 1 {
            (component, Some(u32::from(dimensions[0])))
        } else {
            (
                self.symbolic_shape(component, vec![dimensions[1]], span),
                Some(u32::from(dimensions[0])),
            )
        }
    }
}
