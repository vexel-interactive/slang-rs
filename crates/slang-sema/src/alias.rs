use crate::{
    Type,
    checker::{Checker, span_key},
};
use slang_source::Span;
use slang_syntax::{Declaration, Module};
use std::collections::{BTreeMap, BTreeSet};

impl Checker {
    pub(crate) fn declare_types(&mut self, module: &Module) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for declaration in &module.declarations {
            let (name, modifiers, span) = match declaration {
                Declaration::Struct(s) => {
                    self.struct_definitions.insert(s.name.clone(), s.clone());
                    self.structures.insert(s.name.clone(), BTreeMap::new());
                    (&s.name, &s.modifiers, s.span)
                }
                Declaration::Interface(i) => {
                    self.interface_definitions.insert(i.name.clone(), i.clone());
                    (&i.name, &i.modifiers, i.span)
                }
                Declaration::TypeAlias(a) => {
                    self.aliases.insert(a.name.clone(), a.target.clone());
                    (&a.name, &a.modifiers, a.span)
                }
                _ => continue,
            };
            if Type::builtin(self.source_name(name)).is_some() || !names.insert(name.clone()) {
                self.error(
                    "T003",
                    span,
                    format!("duplicate type '{}'", self.source_name(name)),
                );
            }
            let owner = self
                .symbols
                .get(name)
                .map_or(slang_module::ModuleId(0), |o| o.module);
            self.source_types
                .insert((owner, self.source_name(name).to_owned()));
            if modifiers.iter().any(|m| m == "public") {
                self.public_types.insert(name.clone());
            }
        }
        self.declare_static_members(module);
        self.declare_nested_types(module);
        for declaration in &module.declarations {
            if let Declaration::TypeAlias(alias) = declaration {
                self.check_visibility_modifiers(&alias.modifiers, alias.span);
                let ty = self.resolve_alias(&alias.name, alias.span);
                if alias.modifiers.iter().any(|m| m == "public") {
                    self.require_public_type(&ty, alias.span);
                }
                self.declaration_types.insert(span_key(alias.span), ty);
            }
        }
        names
    }

    pub(crate) fn resolve_alias(&mut self, name: &str, span: Span) -> Type {
        if let Some(ty) = self.alias_types.get(name) {
            return ty.clone();
        }
        if self.resolving_aliases.len() >= 32 || !self.resolving_aliases.insert(name.into()) {
            self.error(
                "T002",
                span,
                format!(
                    "cyclic or excessively nested type alias '{}'",
                    self.source_name(name)
                ),
            );
            return Type::Error;
        }
        let target = self.aliases[name].clone();
        let previous_module = self.current_module;
        let previous_type = self.current_type.take();
        let previous_static_type = self.current_static_type.take();
        self.current_static_type = self.type_owners.get(name).cloned();
        self.current_module = self
            .symbols
            .get(name)
            .map_or(slang_module::ModuleId(0), |o| o.module);
        let ty = self.resolve_type(&target);
        let major = self.reference_matrix_major(&target);
        self.alias_matrix_layouts.insert(name.into(), major);
        self.current_module = previous_module;
        self.current_type = previous_type;
        self.current_static_type = previous_static_type;
        self.resolving_aliases.remove(name);
        self.alias_types.insert(name.into(), ty.clone());
        ty
    }
}
