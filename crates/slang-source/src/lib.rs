//! Source ownership and diagnostics shared by all compiler phases.

mod diagnostic;
mod source;

pub use diagnostic::{Diagnostic, Severity};
pub use source::{LineDirective, SourceFile, SourceId, SourceLocation, SourceMap, Span};
