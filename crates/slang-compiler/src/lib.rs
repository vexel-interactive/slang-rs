//! Native Rust compilation without linking or executing the upstream compiler.
//!
//! ```
//! use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
//! let result = compile_source("example.slang", "float twice(float x) { return x * 2.0; }",
//!     &CompileOptions::default(), &MemoryIncludes::default()).unwrap();
//! assert!(result.artifact.unwrap().contains("float twice(float x)"));
//! ```

mod compiler;
mod include;
mod profile;
mod reflection;

pub use compiler::{Compilation, CompileError, CompileOptions, compile_file, compile_source};
pub use include::{FileIncludes, MemoryIncludes};
pub use profile::ShaderProfile;
pub use reflection::{EntryPointInfo, GlobalInfo, Reflection};
pub use slang_codegen::Target;
pub use slang_preprocessor::IncludeResolver;
pub use slang_source::{Diagnostic, Severity, SourceMap};
