//! Discover constructor specializations from calls and contextual initialization.

use super::*;
use crate::{
    Type,
    checker::{Checker, Signature},
};

pub(crate) type ConstructorKey = (String, Vec<String>);

#[derive(Clone)]
pub(crate) struct ConstructorRequest {
    pub types: Vec<Type>,
    pub span: Span,
    pub module: slang_module::ModuleId,
}

impl Checker {
    pub(crate) fn inferred_constructor_overloads(
        &self,
        owner: &str,
        types: &[Type],
        span: Span,
    ) -> Result<Option<&[Signature]>, Diagnostic> {
        if !self.generic_constructors.contains_key(owner) {
            return Ok(None);
        }
        let key = (
            owner.to_owned(),
            types.iter().map(ToString::to_string).collect(),
        );
        if let Some(name) = self.constructor_applications.get(&key) {
            return Ok(Some(self.functions.get(name).map_or(&[], Vec::as_slice)));
        }
        if self.infer_generic_constructors {
            self.generic_constructor_requests
                .borrow_mut()
                .entry(key)
                .or_insert_with(|| ConstructorRequest {
                    types: types.to_vec(),
                    span,
                    module: self.current_module,
                });
        }
        Err(Diagnostic::error(
            "T002",
            span,
            "cannot infer generic constructor arguments",
        ))
    }

    pub(crate) fn memberwise_constructor_score(
        &self,
        owner: &str,
        arguments: &[Expr],
        types: &[Type],
    ) -> Option<usize> {
        if !self.generic_constructors.get(owner)?.synthesize_members {
            return None;
        }
        let structure = &self.struct_definitions[owner];
        let public = self.public_types.contains(owner);
        let fields = structure
            .fields
            .iter()
            .filter(|f| !public || f.modifiers.iter().any(|m| m == "public"))
            .collect::<Vec<_>>();
        if arguments.len() > fields.len() {
            return None;
        }
        let mut cost = 0;
        for ((argument, actual), field) in arguments.iter().zip(types).zip(fields) {
            let target = &self.structures[owner][&field.name];
            if !self.can_initialize(argument, target) {
                return None;
            }
            cost += actual.literal_conversion_cost(
                target,
                slang_syntax::integer_literal(argument).ok().flatten(),
            );
        }
        Some(cost)
    }
}
