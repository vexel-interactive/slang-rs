//! Typed structured IR, scalar SSA analysis, and target lowering.

mod array_result;
mod atomic;
mod constant_buffer_value;
mod derivative;
mod empty_array;
pub mod flow;
mod geometric;
mod initialization;
mod integer;
mod intrinsic;
mod lower;
mod matrix_resource;
mod matrix_storage;
mod model;
mod packed_uniform;
mod scope;
mod static_local;
mod subword_uniform;
mod uniform;
mod visit;
mod wide_arithmetic;

pub use lower::{lower, lower_with_native_16bit};
pub use model::*;
