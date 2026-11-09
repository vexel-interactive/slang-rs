use crate::{Type, checker::Checker};
use slang_source::Span;
use slang_syntax::{Declaration, Module, TypeRef};
use std::collections::BTreeSet;

impl Checker {
    pub(crate) fn declare_nested_types(&mut self, module: &Module) {
        let mut names = BTreeSet::new();
        for member in &module.nested_types {
            let key = (member.owner.clone(), member.name.clone());
            if !names.insert(key.clone())
                || self.struct_definitions.get(&member.owner).is_some_and(|s| s.fields.iter().any(|f| f.name == member.name))
                || module.static_members.iter().any(|m| m.owner == member.owner && m.name == member.name)
                || module.declarations.iter().any(|d| matches!(d, Declaration::Function(f) if f.method.as_ref().is_some_and(|m| m.owner == member.owner && m.name == member.name)))
            {
                self.error("T003", member.span, "nested type name conflicts with another member");
            }
            if self.public_types.contains(&member.type_name) {
                self.public_fields.insert(key);
            }
        }
    }

    pub(crate) fn type_name(&mut self, name: &str, span: Span) -> Option<String> {
        let mut parts = name.split("::");
        let first = parts.next().unwrap();
        let mut target = first.to_owned();
        if let Some(current) = &self.current_static_type
            && let Some(owner) = self.lexical_member_owner(current, first)
        {
            if let Some(nested) = self.nested_types.get(&(owner, first.into())) {
                target = nested.clone();
            } else if !name.contains("::") || self.lookup(first).is_none() {
                self.error(
                    "T002",
                    span,
                    format!("member '{first}' does not name a type"),
                );
                return None;
            }
        }
        for member in parts {
            let ty = if let Some(binding) = self.lookup(&target) {
                match &binding.ty {
                    Type::ConstantBuffer { element, .. } => *element.clone(),
                    ty => ty.clone(),
                }
            } else {
                self.resolve_type(&Self::named_type(&target, span))
            };
            let Type::Struct(owner) = ty else {
                self.error("T002", span, "nested type lookup requires a struct owner");
                return None;
            };
            let Some(nested) = self
                .nested_types
                .get(&(owner.clone(), member.into()))
                .cloned()
            else {
                self.error("T002", span, format!("unknown nested type '{member}'"));
                return None;
            };
            if !self.field_accessible(&owner, member) {
                self.error(
                    "T018",
                    span,
                    format!("type '{member}' is internal to its module"),
                );
                return None;
            }
            target = nested;
        }
        Some(target)
    }

    /// Search enclosing declaration scopes, stopping at the nearest member.
    pub(crate) fn lexical_member_owner(&self, owner: &str, name: &str) -> Option<String> {
        let mut owner = Some(owner);
        while let Some(current) = owner {
            let key = (current.to_owned(), name.to_owned());
            if self
                .struct_definitions
                .get(current)
                .is_some_and(|s| s.fields.iter().any(|f| f.name == name))
                || self.static_fields.contains_key(&key)
                || self.methods.contains_key(&key)
                || self.nested_types.contains_key(&key)
            {
                return Some(current.to_owned());
            }
            owner = self.type_owners.get(current).map(String::as_str);
        }
        None
    }

    pub(crate) fn nested_type(&mut self, owner: &str, name: &str, span: Span) -> Option<Type> {
        let target = self.nested_types.get(&(owner.into(), name.into()))?.clone();
        if !self.field_accessible(owner, name) {
            self.error(
                "T018",
                span,
                format!("type '{name}' is internal to its module"),
            );
            return Some(Type::Error);
        }
        Some(self.resolve_type(&Self::named_type(&target, span)))
    }

    fn named_type(name: &str, span: Span) -> TypeRef {
        TypeRef {
            name: name.into(),
            base: None,
            application: false,
            owner_ref: false,
            parameter_ref: false,
            arguments: vec![],
            matrix_major: None,
            arrays: vec![],
            span,
        }
    }
}
