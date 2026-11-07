use serde::Serialize;
use slang_source::SourceId;
use slang_syntax::Module;
use std::collections::BTreeMap;

/// Index within a single program's module graph. The root always has index zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ModuleId(pub usize);

#[derive(Clone, Debug, Serialize)]
pub struct ModuleInfo {
    pub id: ModuleId,
    pub name: String,
    pub path: String,
    pub source: SourceId,
    pub legacy_visibility: bool,
    /// Primary source and explicit __include members, excluding preprocessor headers.
    pub files: Vec<SourceId>,
    pub imports: Vec<ModuleId>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SymbolOrigin {
    pub module: ModuleId,
    pub name: String,
}

/// One AST with unique expression IDs and declaration names, plus source ownership.
#[derive(Clone, Debug)]
pub struct Program {
    pub syntax: Module,
    pub modules: Vec<ModuleInfo>,
    pub symbols: BTreeMap<String, SymbolOrigin>,
    pub diagnostics: Vec<slang_source::Diagnostic>,
}

pub(crate) struct Node {
    pub info: ModuleInfo,
    pub syntax: Module,
    pub dependencies: Vec<(ModuleId, bool)>,
}
