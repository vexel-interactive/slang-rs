//! Module loading uses caller-supplied source access and owns no global cache.
//! Each imported file is preprocessed independently; linking preserves lexical scopes.

mod graph;
mod link;
mod loader;
mod resolve;

pub use graph::{ModuleId, ModuleInfo, Program, SymbolOrigin};
pub use loader::load;
