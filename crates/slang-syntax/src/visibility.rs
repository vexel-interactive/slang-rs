use crate::{Declaration, Module};

impl Module {
    /// Legacy sources without module directives or access modifiers export all declarations.
    pub fn uses_legacy_visibility(&self) -> bool {
        let explicit = |modifiers: &[String]| {
            modifiers
                .iter()
                .any(|m| matches!(m.as_str(), "public" | "internal" | "private"))
        };
        !self.module_declared
            && self.name.is_none()
            && self.implementation.is_none()
            && self.file_includes.is_empty()
            && !self
                .declarations
                .iter()
                .any(|declaration| match declaration {
                    Declaration::Struct(s) => {
                        explicit(&s.modifiers) || s.fields.iter().any(|f| explicit(&f.modifiers))
                    }
                    Declaration::Function(f) => {
                        explicit(&f.modifiers)
                            || f.parameters.iter().any(|p| explicit(&p.modifiers))
                    }
                    Declaration::Global(v) => explicit(&v.modifiers),
                    Declaration::TypeAlias(a) => explicit(&a.modifiers),
                })
    }

    /// Make the legacy default explicit before declarations from different modules are linked.
    pub fn apply_legacy_visibility(&mut self) {
        if !self.uses_legacy_visibility() {
            return;
        }
        for declaration in &mut self.declarations {
            let modifiers = match declaration {
                Declaration::Struct(s) => {
                    for field in &mut s.fields {
                        field.modifiers.push("public".into());
                    }
                    &mut s.modifiers
                }
                Declaration::Function(f) => &mut f.modifiers,
                Declaration::Global(v) => &mut v.modifiers,
                Declaration::TypeAlias(a) => &mut a.modifiers,
            };
            modifiers.push("public".into());
        }
    }
}
