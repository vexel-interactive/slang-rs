//! Typed structured IR. This is the first lowering layer, not an SSA optimizer.

mod atomic;
mod derivative;
mod geometric;
mod initialization;
mod intrinsic;
mod lower;
mod matrix_resource;
mod matrix_storage;
mod model;
mod packed_uniform;
mod scope;
mod static_local;
mod uniform;
mod visit;

pub use lower::lower;
pub use model::*;
