use crate::{
    Type,
    checker::{Checker, Storage},
};
use slang_syntax::{Declaration, Expr, ExprKind, Module};
use std::collections::BTreeSet;

impl Checker {
    pub(crate) fn declare_static_members(&mut self, module: &Module) {
        let mut names = BTreeSet::new();
        for member in &module.static_members {
            let key = (member.owner.clone(), member.name.clone());
            let conflict = !names.insert(key.clone())
                || self.struct_definitions.get(&member.owner).is_some_and(|s| s.fields.iter().any(|f| f.name == member.name))
                || module.declarations.iter().any(|d| matches!(d, Declaration::Function(f) if f.method.as_ref().is_some_and(|m| m.owner == member.owner && m.name == member.name)));
            if conflict {
                self.error(
                    "T003",
                    member.span,
                    "static field name conflicts with another member",
                );
            }
            if module.declarations.iter().any(|d| matches!(d, Declaration::Global(v) if v.span == member.span && v.modifiers.iter().any(|m| m == "public"))) {
                self.public_fields.insert(key);
                if !self.public_types.contains(&member.owner) {
                    self.error("T018", member.span, "a public static field requires a public struct");
                }
            }
        }
    }

    /// Type-check member lookup without introducing a value load of its base.
    pub(crate) fn member_owner(&mut self, base: &Expr) -> Option<(String, bool)> {
        if let ExprKind::Name(name) = &base.kind
            && name == "this"
            && self.lookup(name).is_none()
            && let Some(owner) = self.current_static_type.clone()
        {
            self.expression_types
                .insert(base.id, Type::Meta(Box::new(Type::Struct(owner.clone()))));
            return Some((owner, true));
        }
        if matches!(base.kind, ExprKind::Name(_) | ExprKind::TypeApplication(_))
            && let Ok(Some(Type::Struct(owner))) = self.constructor_target(base)
        {
            self.expression_types
                .insert(base.id, Type::Meta(Box::new(Type::Struct(owner.clone()))));
            return Some((owner, true));
        }
        match self.expression(base) {
            Type::Struct(owner) => Some((owner, false)),
            Type::ConstantBuffer { element, .. } => match *element {
                Type::Struct(owner) => Some((owner, false)),
                _ => None,
            },
            Type::Meta(ty) => match *ty {
                Type::Struct(owner) => Some((owner, true)),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn static_field(
        &mut self,
        expression: &Expr,
        owner: &str,
        name: &str,
    ) -> Option<Type> {
        let declaring_owner = if matches!(expression.kind, ExprKind::Name(_)) {
            self.lexical_member_owner(owner, name)?
        } else {
            owner.to_owned()
        };
        let target = self
            .static_fields
            .get(&(declaring_owner.clone(), name.into()))?
            .clone();
        if !self.field_accessible(&declaring_owner, name) {
            self.error(
                "T018",
                expression.span,
                format!("field '{name}' is internal to its module"),
            );
            return Some(Type::Error);
        }
        let binding = self.lookup(&target).cloned()?;
        if let Storage::Global(key) | Storage::GroupShared(key) = binding.storage {
            self.global_references.insert(expression.id, key);
        }
        self.static_references.insert(expression.id, target);
        Some(binding.ty)
    }

    pub(crate) fn declaration_static_owner(&mut self, declaration: &Declaration) {
        self.current_static_type = match declaration {
            Declaration::Struct(s) => Some(s.name.clone()),
            Declaration::Global(v) => self.static_owners.get(&v.name).cloned(),
            Declaration::Function(f) => f.owner_type().map(str::to_owned),
            Declaration::TypeAlias(a) => self.type_owners.get(&a.name).cloned(),
        };
    }

    pub(crate) fn static_declaration(&self, name: &str) -> bool {
        self.static_owners.contains_key(name)
    }
}
