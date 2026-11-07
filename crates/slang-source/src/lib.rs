//! Source ownership and diagnostics shared by all compiler phases.

mod diagnostic;
mod source;

pub use diagnostic::{Diagnostic, Severity};
pub use source::{SourceFile, SourceId, SourceMap, Span};
