//! Typed structured IR. This is the first lowering layer, not an SSA optimizer.

mod atomic;
mod initialization;
mod intrinsic;
mod lower;
mod model;
mod scope;
mod static_local;
mod visit;

pub use lower::lower;
pub use model::*;
