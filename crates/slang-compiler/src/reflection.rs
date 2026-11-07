use serde::Serialize;
use slang_ir::Module;
use slang_sema::{EntryPoint, Type};

/// Declaration reflection only. GPU binding allocation and byte layout are not yet implemented.
#[derive(Clone, Debug, Serialize)]
pub struct Reflection {
    pub schema_version: u32,
    pub entry_points: Vec<EntryPoint>,
    pub globals: Vec<GlobalInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GlobalInfo {
    pub name: String,
    pub type_name: String,
    pub resource_access: Option<String>,
}

impl Reflection {
    pub(crate) fn from_module(module: &Module) -> Self {
        Self {
            schema_version: 2,
            entry_points: module.entry_points.clone(),
            globals: module
                .globals
                .iter()
                .map(|global| GlobalInfo {
                    name: global.name.clone(),
                    type_name: global.ty.to_string(),
                    resource_access: match &global.ty {
                        Type::Buffer { writable, .. } => {
                            Some(if *writable { "read_write" } else { "read" }.into())
                        }
                        _ => None,
                    },
                })
                .collect(),
        }
    }
}
