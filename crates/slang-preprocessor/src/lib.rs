//! A session-local preprocessor. Include access is supplied by the caller.

mod condition;
mod expand;
mod line;
mod macros;
mod processor;
mod substitute;

pub use processor::{IncludeResolver, Options, Preprocessed, preprocess};
