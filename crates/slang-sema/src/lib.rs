//! Semantic checking is target-independent and never accepts error types as output.

mod alias;
mod call;
mod checker;
mod constant;
mod declaration;
mod entry;
mod expression;
mod interface;
mod intrinsic;
mod resource;
mod statement;
mod types;
mod visibility;

pub use checker::{CheckedModule, EntryPoint, ShaderStage, check, check_program, span_key};
pub use types::{Scalar, TextureDimension, Type};
