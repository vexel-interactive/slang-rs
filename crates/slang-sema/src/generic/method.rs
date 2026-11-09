//! Preserve member receivers while inferring and specializing method arguments.

use super::*;
use crate::{Type, checker::Checker};

impl Specializer {
    pub(super) fn normalize_member_application(
        &mut self,
        expression: &mut Expr,
    ) -> Result<(), Diagnostic> {
        let ExprKind::GenericApplication { callee, .. } = &expression.kind else {
            return Ok(());
        };
        if let ExprKind::Member { base, name, .. } = &callee.kind
            && self.method_names.contains(name)
        {
            let mut owner = base.type_reference();
            let nested = if let Some(owner) = &mut owner {
                let mut root = &*owner;
                while let Some(base) = &root.base {
                    root = base;
                }
                let root = root.name.split("::").next().unwrap();
                if self.templates.get(root).is_some_and(|t| !t.function)
                    || self.aliases.contains_key(root)
                    || self.shapes.structures.contains_key(root)
                {
                    self.ty(owner)?;
                    self.canonical(owner, 0)?;
                    self.nested
                        .contains_key(&(owner.name.clone(), name.clone()))
                } else {
                    false
                }
            } else {
                false
            };
            if !nested {
                return Ok(());
            }
        }
        if let Some(reference) = expression.type_reference() {
            expression.kind = ExprKind::TypeApplication(Box::new(reference));
        }
        Ok(())
    }

    pub(super) fn member_receiver(&mut self, callee: &mut Expr) -> Result<(), Diagnostic> {
        let callee = match &mut callee.kind {
            ExprKind::GenericApplication { callee, .. } => callee,
            _ => callee,
        };
        if let ExprKind::Member { base, .. } = &mut callee.kind {
            self.expression(base)?;
        }
        Ok(())
    }
}

impl Checker {
    pub(crate) fn pending_member_call(&self, expression: &Expr) -> bool {
        let callee = match &expression.kind {
            ExprKind::GenericApplication { callee, .. } => callee,
            _ => expression,
        };
        let name = match &callee.kind {
            ExprKind::Member { .. } => return true,
            ExprKind::Name(name) => name,
            ExprKind::TypeApplication(reference) if reference.base.is_none() => &reference.name,
            _ => return false,
        };
        self.current_type
            .as_ref()
            .or(self.current_static_type.as_ref())
            .is_some_and(|owner| self.lexical_member_owner(owner, name).is_some())
    }

    pub(crate) fn missing_generic_member(&mut self, expression: &Expr, span: Span) {
        let callee = match &expression.kind {
            ExprKind::GenericApplication { callee, .. } => callee,
            _ => expression,
        };
        if let ExprKind::Member { base, name, .. } = &callee.kind {
            if self
                .expression_types
                .get(&base.id)
                .is_some_and(|t| *t != Type::Error)
            {
                self.error("T014", span, format!("no callable generic member '{name}'"));
            }
        } else {
            self.error("T014", span, "member is not callable");
        }
    }

    pub(crate) fn generic_method_source(&mut self, expression: &Expr) -> Option<String> {
        let callee = match &expression.kind {
            ExprKind::GenericApplication { callee, .. } => callee,
            _ => expression,
        };
        let (owner, name) = match &callee.kind {
            ExprKind::Member { base, name, .. } => (self.member_owner(base)?.0, name),
            ExprKind::Name(name) => {
                let current = self
                    .current_type
                    .as_ref()
                    .or(self.current_static_type.as_ref())?;
                (self.lexical_member_owner(current, name)?, name)
            }
            ExprKind::TypeApplication(reference) if reference.base.is_none() => {
                let current = self
                    .current_type
                    .as_ref()
                    .or(self.current_static_type.as_ref())?;
                (
                    self.lexical_member_owner(current, &reference.name)?,
                    &reference.name,
                )
            }
            _ => return None,
        };
        let key = (owner, name.clone());
        self.generic_methods
            .get(&key)
            .or_else(|| self.methods.get(&key))
            .cloned()
    }
}
