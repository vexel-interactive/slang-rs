use crate::Module;
use slang_layout::{
    UniformFieldLayout, UniformLayoutError, UniformPacker, UniformTypeLayout, UniformVariableLayout,
};
use slang_sema::{CheckedModule, Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn lower(module: &mut Module, checked: &CheckedModule) -> Result<(), Diagnostic> {
    let structures: BTreeMap<_, _> = module
        .structures
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let mut layouts = BTreeMap::new();
    let mut uniforms = vec![];
    for (index, global) in module.globals.iter().enumerate() {
        if global.is_global_uniform() {
            calculate(&global.ty, &structures, &mut layouts).map_err(|e| error(global.span, e))?;
            uniforms.push(index);
        } else {
            let mut ty = &global.ty;
            while let Type::Array(element, _) = ty {
                ty = element;
            }
            if let Type::ConstantBuffer(element) = ty {
                calculate(element, &structures, &mut layouts).map_err(|e| error(global.span, e))?;
            }
        }
    }
    // Parameter layout starts with the root module, then discovered imports.
    uniforms.sort_by_key(|&i| {
        checked
            .symbols
            .get(&module.globals[i].name)
            .map_or(0, |o| o.module.0)
    });
    let mut offsets = vec![];
    let mut packer = UniformPacker::default();
    for &index in &uniforms {
        let global = &module.globals[index];
        let name = global.ty.to_string();
        let Some(layout) = layouts.get(&name).and_then(Option::as_ref) else {
            // Resource-valued aggregates need a separate mixed-layout model.
            offsets.clear();
            break;
        };
        let offset = packer.push(layout).map_err(|e| error(global.span, e))?;
        offsets.push((
            index,
            UniformVariableLayout {
                offset,
                type_name: name,
            },
        ));
    }
    if !uniforms.is_empty() && offsets.len() == uniforms.len() {
        module.global_uniform_size = Some(packer.finish(vec![]).size);
        for (index, layout) in offsets {
            module.globals[index].uniform_layout = Some(layout);
        }
    }
    for global in &mut module.globals {
        let mut ty = &global.ty;
        while let Type::Array(element, _) = ty {
            ty = element;
        }
        if let Type::ConstantBuffer(element) = ty {
            let name = element.to_string();
            if layouts.get(&name).is_some_and(Option::is_some) {
                global.uniform_layout = Some(UniformVariableLayout {
                    offset: 0,
                    type_name: name,
                });
            }
        }
    }
    module.uniform_types = layouts
        .into_iter()
        .filter_map(|(name, layout)| layout.map(|layout| (name, layout)))
        .collect();
    Ok(())
}

/// Visit the type graph explicitly, memoizing layouts rather than expanding paths.
fn calculate<'a>(
    root: &'a Type,
    structures: &BTreeMap<&str, &'a crate::Struct>,
    layouts: &mut BTreeMap<String, Option<UniformTypeLayout>>,
) -> Result<(), UniformLayoutError> {
    let mut stack = vec![(root, false)];
    let mut active = BTreeSet::new();
    while let Some((ty, exiting)) = stack.pop() {
        let name = ty.to_string();
        if layouts.contains_key(&name) {
            continue;
        }
        if !exiting {
            if !active.insert(name.clone()) {
                return Err(UniformLayoutError::InvalidShape);
            }
            stack.push((ty, true));
            match ty {
                Type::Array(element, _) => stack.push((element, false)),
                Type::Struct(name) => {
                    let structure = structures
                        .get(name.as_str())
                        .ok_or(UniformLayoutError::InvalidShape)?;
                    stack.extend(structure.fields.iter().rev().map(|f| (&f.ty, false)));
                }
                _ => {}
            }
            continue;
        }
        active.remove(&name);
        let layout = match ty {
            Type::Scalar(scalar) => Some(UniformTypeLayout::numeric(component_bytes(*scalar), 1)?),
            Type::Vector(scalar, width) => Some(UniformTypeLayout::numeric(
                component_bytes(*scalar),
                *width,
            )?),
            Type::Matrix(scalar, rows, columns) => Some(UniformTypeLayout::matrix(
                component_bytes(*scalar),
                *rows,
                *columns,
            )?),
            Type::Array(element, count) => layouts
                .get(&element.to_string())
                .and_then(Option::as_ref)
                .map(|layout| layout.array(element.to_string(), *count))
                .transpose()?,
            Type::Struct(name) => {
                let structure = structures[name.as_str()];
                let mut packer = UniformPacker::default();
                let mut fields = vec![];
                let mut supported = true;
                for field in &structure.fields {
                    let name = field.ty.to_string();
                    let Some(layout) = layouts.get(&name).and_then(Option::as_ref) else {
                        supported = false;
                        break;
                    };
                    fields.push(UniformFieldLayout {
                        name: field.name.clone(),
                        offset: packer.push(layout)?,
                        type_name: name,
                    });
                }
                supported.then(|| packer.finish(fields))
            }
            _ => None,
        };
        layouts.insert(name, layout);
    }
    Ok(())
}

fn component_bytes(scalar: Scalar) -> u32 {
    // HLSL half uses 32-bit storage without DXC's enable-16bit-types option.
    if scalar == Scalar::Double { 8 } else { 4 }
}

fn error(span: Span, error: UniformLayoutError) -> Diagnostic {
    Diagnostic::error("I002", span, error.to_string())
}
