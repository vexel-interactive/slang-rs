//! Typed structured IR. This is the first lowering layer, not an SSA optimizer.

mod initialization;
mod lower;
mod model;

pub use lower::lower;
pub use model::*;
