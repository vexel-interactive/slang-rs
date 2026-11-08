use crate::{Type, checker::Checker};
use slang_source::{SourceId, Span};
use std::collections::{BTreeMap, BTreeSet};

impl Checker {
    /// Run after every struct field and local declaration has been checked.
    pub(crate) fn validate_constant_buffers(&mut self) {
        let buffers: Vec<_> = self
            .declaration_types
            .iter()
            .filter_map(|(span, ty)| {
                let mut ty = ty;
                while let Type::Array(element, _) = ty {
                    ty = element;
                }
                if let Type::ConstantBuffer { element, .. } = ty {
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

    pub(crate) fn constant_buffer_data(&self, ty: &Type, path: &mut BTreeSet<String>) -> bool {
        self.data_type(ty, path, &mut BTreeMap::new())
    }

    fn data_type(
        &self,
        ty: &Type,
        path: &mut BTreeSet<String>,
        cache: &mut BTreeMap<String, bool>,
    ) -> bool {
        match ty {
            Type::Void | Type::Scalar(_) | Type::Vector(..) | Type::Matrix(..) | Type::Error => {
                true
            }
            Type::Array(element, _) => self.data_type(element, path, cache),
            Type::Struct(name) => {
                if path.len() >= 128 || !path.insert(name.clone()) {
                    return false;
                }
                if let Some(valid) = cache.get(name) {
                    path.remove(name);
                    return *valid;
                }
                let valid = self.structures.get(name).is_some_and(|fields| {
                    fields
                        .values()
                        .all(|field| self.data_type(field, path, cache))
                });
                path.remove(name);
                cache.insert(name.clone(), valid);
                valid
            }
            _ => false,
        }
    }
}
