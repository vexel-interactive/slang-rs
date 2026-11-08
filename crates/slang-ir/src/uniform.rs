use crate::Module;
use slang_layout::{
    MatrixMajor, UniformFieldLayout, UniformLayoutError, UniformPacker, UniformTypeLayout,
    UniformVariableLayout,
};
use slang_sema::{CheckedModule, Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn lower(module: &mut Module, checked: &CheckedModule) -> Result<(), Diagnostic> {
    let default_major = module.matrix_major;
    let structures: BTreeMap<_, _> = module
        .structures
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let mut layouts = BTreeMap::new();
    let mut uniforms = vec![];
    for (index, global) in module.globals.iter().enumerate() {
        if global.is_global_uniform() {
            calculate(
                &global.ty,
                row_major(global, default_major),
                default_major,
                &structures,
                &mut layouts,
            )
            .map_err(|e| error(global.span, e))?;
            uniforms.push(index);
        } else {
            let mut ty = &global.ty;
            while let Type::Array(element, _) = ty {
                ty = element;
            }
            if let Type::ConstantBuffer {
                element,
                matrix_major,
            } = ty
            {
                calculate(
                    element,
                    matrix_major.unwrap_or(default_major) == MatrixMajor::Row,
                    default_major,
                    &structures,
                    &mut layouts,
                )
                .map_err(|e| error(global.span, e))?;
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
        let name = layout_name(&global.ty, row_major(global, default_major));
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
        if let Type::ConstantBuffer {
            element,
            matrix_major,
        } = ty
        {
            let name = layout_name(
                element,
                matrix_major.unwrap_or(default_major) == MatrixMajor::Row,
            );
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
    row: bool,
    default_major: MatrixMajor,
    structures: &BTreeMap<&str, &'a crate::Struct>,
    layouts: &mut BTreeMap<String, Option<UniformTypeLayout>>,
) -> Result<(), UniformLayoutError> {
    let mut stack = vec![(root, row, false)];
    let mut active = BTreeSet::new();
    while let Some((ty, row, exiting)) = stack.pop() {
        let name = layout_name(ty, row);
        if layouts.contains_key(&name) {
            continue;
        }
        if !exiting {
            if !active.insert(name.clone()) {
                return Err(UniformLayoutError::InvalidShape);
            }
            stack.push((ty, row, true));
            match ty {
                Type::Array(_, 0) => {}
                Type::Array(element, _) => stack.push((element, row, false)),
                Type::Struct(name) => {
                    let structure = structures
                        .get(name.as_str())
                        .ok_or(UniformLayoutError::InvalidShape)?;
                    stack.extend(
                        structure
                            .fields
                            .iter()
                            .rev()
                            .map(|f| (&f.ty, row_major(f, default_major), false)),
                    );
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
            Type::Matrix(scalar, rows, columns) => Some(UniformTypeLayout::matrix_with_major(
                component_bytes(*scalar),
                *rows,
                *columns,
                if row {
                    MatrixMajor::Row
                } else {
                    MatrixMajor::Column
                },
            )?),
            Type::Array(element, 0) => {
                Some(UniformTypeLayout::empty_array(layout_name(element, row)))
            }
            Type::Array(element, count) => layouts
                .get(&layout_name(element, row))
                .and_then(Option::as_ref)
                .map(|layout| layout.array(layout_name(element, row), *count))
                .transpose()?,
            Type::Struct(name) => {
                let structure = structures[name.as_str()];
                let mut packer = UniformPacker::default();
                let mut fields = vec![];
                let mut supported = true;
                for field in &structure.fields {
                    if let Some(offset) = field.pack_offset {
                        let name = layout_name(&field.ty, row_major(field, default_major));
                        if let Some(layout) = layouts.get(&name).and_then(Option::as_ref) {
                            packer.place(offset, layout)?;
                        }
                    }
                }
                for field in &structure.fields {
                    let name = layout_name(&field.ty, row_major(field, default_major));
                    let Some(layout) = layouts.get(&name).and_then(Option::as_ref) else {
                        supported = false;
                        break;
                    };
                    fields.push(UniformFieldLayout {
                        name: field.name.clone(),
                        offset: match field.pack_offset {
                            Some(offset) => offset,
                            None => packer.push(layout)?,
                        },
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

fn row_major(value: &crate::Variable, default_major: MatrixMajor) -> bool {
    value
        .modifiers
        .iter()
        .rev()
        .find(|m| matches!(m.as_str(), "row_major" | "column_major"))
        .map_or(default_major == MatrixMajor::Row, |m| m == "row_major")
}

fn layout_name(ty: &Type, row: bool) -> String {
    let mut element = ty;
    while let Type::Array(inner, _) = element {
        element = inner;
    }
    if row && matches!(element, Type::Matrix(..)) {
        format!("row_major {ty}")
    } else {
        ty.to_string()
    }
}

fn component_bytes(scalar: Scalar) -> u32 {
    // HLSL half uses 32-bit storage without DXC's enable-16bit-types option.
    if matches!(scalar, Scalar::Double | Scalar::Int64 | Scalar::Uint64) {
        8
    } else {
        4
    }
}

fn error(span: Span, error: UniformLayoutError) -> Diagnostic {
    Diagnostic::error("I002", span, error.to_string())
}
