use crate::{Type, checker::Checker};
use slang_source::{SourceId, Span};
use std::collections::BTreeSet;

impl Checker {
    /// Run after every struct field and local declaration has been checked.
    pub(crate) fn validate_constant_buffers(&mut self) {
        let buffers: Vec<_> = self
            .declaration_types
            .iter()
            .filter_map(|(span, ty)| {
                if let Type::ConstantBuffer(element) = ty {
                    Some((*span, *element.clone()))
                } else {
                    None
                }
            })
            .collect();
        for ((source, start, end), element) in buffers {
            if !self.constant_buffer_data(&element, &mut BTreeSet::new()) {
                self.error(
                    "T002",
                    Span::new(SourceId(source), start, end),
                    "resource-valued fields in constant buffers are not implemented",
                );
            }
        }
    }

    fn constant_buffer_data(&self, ty: &Type, path: &mut BTreeSet<String>) -> bool {
        match ty {
            Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..) | Type::Error => true,
            Type::Array(element, _) => self.constant_buffer_data(element, path),
            Type::Struct(name) => {
                if path.len() >= 128 || !path.insert(name.clone()) {
                    return false;
                }
                let valid = self.structures.get(name).is_some_and(|fields| {
                    fields
                        .values()
                        .all(|field| self.constant_buffer_data(field, path))
                });
                path.remove(name);
                valid
            }
            _ => false,
        }
    }
}
