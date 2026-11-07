use serde::Serialize;
use slang_codegen::Target;
use slang_ir::Module;
use slang_sema::{ShaderStage, Type};
use std::collections::{BTreeMap, BTreeSet};

/// Declaration and explicit binding reflection. Automatic allocation and byte layout are not yet implemented.
#[derive(Clone, Debug, Serialize)]
pub struct Reflection {
    pub schema_version: u32,
    pub entry_points: Vec<EntryPointInfo>,
    pub globals: Vec<GlobalInfo>,
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
    /// Explicit source binding; absent when automatic allocation would be needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<slang_layout::ResourceBinding>,
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
        let source_globals: BTreeSet<_> = checked
            .module
            .declarations
            .iter()
            .filter_map(|declaration| {
                if let slang_syntax::Declaration::Global(global) = declaration {
                    Some(global.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        Self {
            schema_version: 6,
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
                        .symbols
                        .get(&global.name)
                        .map_or_else(|| global.name.clone(), |o| o.name.clone()),
                    target_name: slang_codegen::entry_point_name(&global.name, target),
                    type_name: global.ty.to_string(),
                    binding: global.binding,
                    resource_access: match resource_element(&global.ty) {
                        Type::ConstantBuffer(_) => Some("read".into()),
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
