use serde::Serialize;
use slang_codegen::Target;
use slang_ir::Module;
use slang_sema::{ShaderStage, Type};
use std::collections::{BTreeMap, BTreeSet};

/// Declaration, resource binding, and legacy HLSL constant-buffer byte layout reflection.
#[derive(Clone, Debug, Serialize)]
pub struct Reflection {
    pub schema_version: u32,
    pub matrix_major: slang_layout::MatrixMajor,
    pub entry_points: Vec<EntryPointInfo>,
    pub globals: Vec<GlobalInfo>,
    /// Shared constant-buffer binding; absent when there are no global uniform values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_uniform_binding: Option<slang_layout::ResourceBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_uniform_size: Option<u64>,
    /// Value layouts, shared by reference through canonical type names.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub uniform_types: BTreeMap<String, slang_layout::UniformTypeLayout>,
    pub modules: Vec<slang_module::ModuleInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EntryPointInfo {
    pub name: String,
    /// Identifier to pass to the target compiler when it differs from the source name.
    pub target_name: String,
    pub stage: ShaderStage,
    /// Profile to pass to an HLSL compiler, including required feature upgrades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_group_size: Option<[u32; 3]>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GlobalInfo {
    pub name: String,
    pub target_name: String,
    pub type_name: String,
    pub resource_access: Option<String>,
    /// Checked binding for a global shader resource; absent for ordinary values and private storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<slang_layout::ResourceBinding>,
    /// Byte offset and value type for a global uniform or ConstantBuffer element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform_layout: Option<slang_layout::UniformVariableLayout>,
    /// Source module for imported globals; absent for root-module globals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

impl Reflection {
    pub(crate) fn from_module(
        module: &Module,
        target: Target,
        checked: &slang_sema::CheckedModule,
        profiles: &BTreeMap<String, crate::ShaderProfile>,
    ) -> Self {
        let buffer_members: BTreeSet<_> = checked
            .module
            .constant_buffers
            .iter()
            .flat_map(|b| &b.members)
            .map(|m| slang_sema::span_key(m.span))
            .collect();
        let source_globals: BTreeSet<_> = checked
            .module
            .declarations
            .iter()
            .filter_map(|declaration| {
                if let slang_syntax::Declaration::Global(global) = declaration {
                    (!buffer_members.contains(&slang_sema::span_key(global.span)))
                        .then_some(global.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        Self {
            schema_version: 10,
            matrix_major: module.matrix_major,
            global_uniform_binding: module.global_uniform_binding,
            global_uniform_size: module.global_uniform_size,
            uniform_types: module.uniform_types.clone(),
            modules: checked.modules.clone(),
            entry_points: module
                .entry_points
                .iter()
                .map(|entry| EntryPointInfo {
                    name: entry.name.clone(),
                    target_name: slang_codegen::entry_point_name(&entry.name, target),
                    stage: entry.stage,
                    target_profile: profiles.get(&entry.name).map(ToString::to_string),
                    thread_group_size: entry.thread_group_size,
                })
                .collect(),
            globals: module
                .globals
                .iter()
                .filter(|global| source_globals.contains(global.name.as_str()))
                .map(|global| GlobalInfo {
                    name: checked
                        .module
                        .constant_buffers
                        .iter()
                        .find(|b| b.variable_name == global.name)
                        .map(|b| b.name.clone())
                        .unwrap_or_else(|| {
                            checked
                                .symbols
                                .get(&global.name)
                                .map_or_else(|| global.name.clone(), |o| o.name.clone())
                        }),
                    target_name: slang_codegen::entry_point_name(
                        if checked.declaration_types[&slang_sema::span_key(global.span)]
                            .is_resource()
                        {
                            &global.name
                        } else {
                            module
                                .uniform_value_functions
                                .get(&global.name)
                                .unwrap_or(&global.name)
                        },
                        target,
                    ),
                    type_name: checked.declaration_types[&slang_sema::span_key(global.span)]
                        .to_string(),
                    binding: global.binding,
                    uniform_layout: global.uniform_layout.clone(),
                    resource_access: match resource_element(
                        &checked.declaration_types[&slang_sema::span_key(global.span)],
                    ) {
                        Type::ConstantBuffer { .. } => Some("read".into()),
                        Type::ByteBuffer { writable }
                        | Type::Buffer { writable, .. }
                        | Type::Texture { writable, .. } => {
                            Some(if *writable { "read_write" } else { "read" }.into())
                        }
                        _ => None,
                    },
                    module: checked
                        .symbols
                        .get(&global.name)
                        .filter(|o| o.module.0 != 0)
                        .map(|o| checked.modules[o.module.0].name.clone()),
                })
                .collect(),
        }
    }
}

fn resource_element(mut ty: &Type) -> &Type {
    while let Type::Array(element, _) = ty {
        ty = element;
    }
    ty
}
