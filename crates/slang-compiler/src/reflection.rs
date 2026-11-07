use serde::Serialize;
use slang_codegen::Target;
use slang_ir::Module;
use slang_sema::{ShaderStage, Type};

/// Declaration reflection only. GPU binding allocation and byte layout are not yet implemented.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_group_size: Option<[u32; 3]>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GlobalInfo {
    pub name: String,
    pub target_name: String,
    pub type_name: String,
    pub resource_access: Option<String>,
    /// Source module for imported globals; absent for root-module globals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

impl Reflection {
    pub(crate) fn from_module(
        module: &Module,
        target: Target,
        checked: &slang_sema::CheckedModule,
    ) -> Self {
        Self {
            schema_version: 4,
            modules: checked.modules.clone(),
            entry_points: module
                .entry_points
                .iter()
                .map(|entry| EntryPointInfo {
                    name: entry.name.clone(),
                    target_name: slang_codegen::entry_point_name(&entry.name, target),
                    stage: entry.stage,
                    thread_group_size: entry.thread_group_size,
                })
                .collect(),
            globals: module
                .globals
                .iter()
                .map(|global| GlobalInfo {
                    name: checked
                        .symbols
                        .get(&global.name)
                        .map_or_else(|| global.name.clone(), |o| o.name.clone()),
                    target_name: slang_codegen::entry_point_name(&global.name, target),
                    type_name: global.ty.to_string(),
                    resource_access: match &global.ty {
                        Type::Buffer { writable, .. } | Type::Texture { writable, .. } => {
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
