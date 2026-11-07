use crate::{FileIncludes, Reflection};
use slang_codegen::Target;
use slang_preprocessor::{IncludeResolver, Options};
use slang_source::{Diagnostic, SourceMap, Span};
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    pub target: Target,
    pub entry_point: Option<String>,
    pub profile: Option<crate::ShaderProfile>,
    pub defines: BTreeMap<String, String>,
    pub include_paths: Vec<PathBuf>,
    pub check_only: bool,
}

#[derive(Clone, Debug)]
pub struct Compilation {
    pub artifact: Option<String>,
    pub ir: slang_ir::Module,
    pub reflection: Reflection,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct CompileError {
    pub diagnostics: Vec<Diagnostic>,
    pub sources: SourceMap,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            writeln!(f, "{}", diagnostic.render(&self.sources))?;
        }
        Ok(())
    }
}
impl std::error::Error for CompileError {}

/// Each call owns all compilation state and can run concurrently with other calls.
pub fn compile_source(
    name: &str,
    text: &str,
    options: &CompileOptions,
    includes: &dyn IncludeResolver,
) -> Result<Compilation, CompileError> {
    let mut sources = SourceMap::default();
    let root = sources.add(name, text.to_owned());
    let mut defines = options.defines.clone();
    defines
        .entry("__SLANG__".into())
        .or_insert_with(|| "1".into());
    let program = slang_module::load(
        root,
        &mut sources,
        includes,
        &Options {
            defines,
            ..Options::default()
        },
    )
    .map_err(|diagnostics| CompileError {
        diagnostics,
        sources: sources.clone(),
    })?;
    let mut warnings = program.diagnostics.clone();
    let checked = slang_sema::check_program(program).map_err(|diagnostics| CompileError {
        diagnostics,
        sources: sources.clone(),
    })?;
    if let Some(entry) = &options.entry_point
        && !checked.entry_points.iter().any(|e| &e.name == entry)
    {
        return Err(CompileError {
            diagnostics: vec![Diagnostic::error(
                "C001",
                Span::new(root, 0, 0),
                format!("shader entry point '{entry}' was not found"),
            )],
            sources,
        });
    }
    let profiles =
        crate::profile::resolve(&checked, options).map_err(|diagnostics| CompileError {
            diagnostics,
            sources: sources.clone(),
        })?;
    warnings.extend(profiles.diagnostics);
    let ir = slang_ir::lower(&checked).map_err(|diagnostic| CompileError {
        diagnostics: vec![diagnostic],
        sources: sources.clone(),
    })?;
    let artifact = if options.check_only {
        None
    } else {
        Some(
            slang_codegen::emit(&ir, options.target).map_err(|diagnostic| CompileError {
                diagnostics: vec![diagnostic],
                sources: sources.clone(),
            })?,
        )
    };
    let reflection = Reflection::from_module(&ir, options.target, &checked, &profiles.entries);
    Ok(Compilation {
        artifact,
        ir,
        reflection,
        sources,
        diagnostics: warnings,
    })
}

pub fn compile_file(
    path: impl AsRef<Path>,
    options: &CompileOptions,
) -> Result<Compilation, CompileError> {
    let path = path.as_ref();
    let path = path.canonicalize().map_err(|e| io_error(path, e))?;
    let text = fs::read_to_string(&path).map_err(|e| io_error(&path, e))?;
    compile_source(
        &crate::include::source_name(&path),
        &text,
        options,
        &FileIncludes {
            search_paths: options.include_paths.clone(),
        },
    )
}

fn io_error(path: &Path, error: std::io::Error) -> CompileError {
    CompileError {
        diagnostics: vec![Diagnostic::error(
            "C002",
            Span::default(),
            format!("{}: {error}", path.display()),
        )],
        sources: SourceMap::default(),
    }
}
