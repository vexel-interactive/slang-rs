use crate::{Type, checker::Checker};
use slang_source::Span;

impl Checker {
    /// Imported modules cannot see declarations from the root without an import.
    pub(crate) fn module_symbol_visible(&self, name: &str) -> bool {
        self.current_module.0 == 0 || self.symbols.get(name).is_none_or(|o| o.module.0 != 0)
    }

    pub(crate) fn source_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.symbols.get(name).map_or(name, |origin| &origin.name)
    }

    pub(crate) fn check_visibility_modifiers(&mut self, modifiers: &[String], span: Span) {
        if modifiers
            .iter()
            .filter(|m| matches!(m.as_str(), "public" | "internal"))
            .count()
            > 1
        {
            self.error("T018", span, "conflicting visibility modifiers");
        }
    }

    pub(crate) fn require_public_type(&mut self, ty: &Type, span: Span) {
        if !self.public_type(ty) {
            self.error(
                "T018",
                span,
                format!("public declaration exposes an internal type {ty}"),
            );
        }
    }

    fn public_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Struct(name) => self.public_types.contains(name),
            Type::Array(element, _)
            | Type::ConstantBuffer { element, .. }
            | Type::Buffer { element, .. }
            | Type::Texture { element, .. } => self.public_type(element),
            _ => true,
        }
    }

    pub(crate) fn field_accessible(&self, structure: &str, field: &str) -> bool {
        let owner = self
            .symbols
            .get(structure)
            .map_or(slang_module::ModuleId(0), |o| o.module);
        owner == self.current_module
            || self
                .public_fields
                .contains(&(structure.to_owned(), field.to_owned()))
    }
}
