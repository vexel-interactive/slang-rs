//! Semantic checking is target-independent and never accepts error types as output.

mod call;
mod checker;
mod declaration;
mod entry;
mod expression;
mod interface;
mod intrinsic;
mod resource;
mod statement;
mod types;

pub use checker::{CheckedModule, EntryPoint, ShaderStage, check, span_key};
pub use types::{Scalar, TextureDimension, Type};
