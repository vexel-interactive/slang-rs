//! Typed structured IR. This is the first lowering layer, not an SSA optimizer.

mod initialization;
mod intrinsic;
mod lower;
mod model;
mod visit;

pub use lower::lower;
pub use model::*;
