//! Semantic checking is target-independent and never accepts error types as output.

mod checker;
mod declaration;
mod entry;
mod expression;
mod statement;
mod types;

pub use checker::{CheckedModule, EntryPoint, check, span_key};
pub use types::{Scalar, Type};
