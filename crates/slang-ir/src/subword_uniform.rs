use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{Scalar, Type};
use slang_source::Diagnostic;
use std::collections::BTreeMap;

/// HLSL packoffset addresses four-byte components and requires register-aligned
/// structs. Native padding expresses a subcomponent without rounding its address.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    if !module.native_16bit_types {
        return Ok(());
    }
    let names = crate::visit::names(module);
    let mut prefix = "_slang_subword_uniform_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut structures = vec![];
    let mut replacements = BTreeMap::new();
    let fields = module
        .constant_buffers
        .iter_mut()
        .flat_map(|buffer| &mut buffer.fields)
        .chain(module.global_uniform_fields.iter_mut().flatten());
    for field in fields {
        let Some(offset) = field.pack_offset.filter(|offset| offset % 4 == 2) else {
            continue;
        };
        let scalar = field
            .ty
            .scalar()
            .filter(|s| matches!(s, Scalar::Half | Scalar::Int16 | Scalar::Uint16))
            .ok_or_else(|| {
                Diagnostic::error(
                    "I002",
                    field.span,
                    "unsupported subcomponent uniform offset",
                )
            })?;
        let name = format!("{prefix}{}", structures.len());
        let value_name = format!("_value_{}", structures.len());
        let mut value = field.clone();
        value.name = value_name.clone();
        value.pack_offset = None;
        let mut fields = vec![];
        let mut remaining = (offset % 16 / 2) as u8;
        while remaining != 0 {
            let width = remaining.min(4);
            let mut padding = value.clone();
            padding.name = format!("_padding_{}_{}", structures.len(), fields.len());
            padding.ty = Type::numeric_shape(scalar, width);
            padding.modifiers.clear();
            fields.push(padding);
            remaining -= width;
        }
        fields.push(value);
        structures.push(Struct {
            name: name.clone(),
            fields,
        });
        field.ty = Type::Struct(name);
        field.pack_offset = Some(offset / 16 * 16);
        replacements.insert(field.name.clone(), (field.ty.clone(), value_name));
    }
    module.structures.extend(structures);
    Lowerer { replacements }.module(module)
}

struct Lowerer {
    replacements: BTreeMap<String, (Type, String)>,
}

impl Visitor for Lowerer {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, expression)?;
        let (ExprKind::Name(name) | ExprKind::GlobalName(name)) = &expression.kind else {
            return Ok(());
        };
        if let Some((ty, value_name)) = self.replacements.get(name) {
            let mut base = expression.clone();
            base.ty = ty.clone();
            expression.kind = ExprKind::Member {
                base: Box::new(base),
                name: value_name.clone(),
            };
        }
        Ok(())
    }
}
