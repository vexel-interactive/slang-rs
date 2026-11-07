use slang_preprocessor::IncludeResolver;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct FileIncludes {
    pub search_paths: Vec<PathBuf>,
}

impl IncludeResolver for FileIncludes {
    fn resolve(
        &self,
        from: &str,
        requested: &str,
        system: bool,
    ) -> Result<(String, String), String> {
        let mut candidates = vec![];
        if !system {
            candidates.push(
                Path::new(from)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(requested),
            );
        }
        candidates.extend(self.search_paths.iter().map(|root| root.join(requested)));
        for candidate in candidates {
            if candidate.is_file() {
                let path = candidate
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", candidate.display()))?;
                let text =
                    fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                return Ok((source_name(&path), text));
            }
        }
        Err(format!("include file '{requested}' was not found"))
    }
}

/// Keep canonical identities readable and usable in source-level file macros.
pub(crate) fn source_name(path: &Path) -> String {
    let name = path.to_string_lossy();
    if let Some(unc) = name.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else if let Some(disk) = name.strip_prefix(r"\\?\")
        && disk.as_bytes().get(1) == Some(&b':')
    {
        disk.into()
    } else {
        name.into_owned()
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryIncludes {
    pub files: BTreeMap<String, String>,
}

impl IncludeResolver for MemoryIncludes {
    fn resolve(&self, _: &str, requested: &str, _: bool) -> Result<(String, String), String> {
        self.files
            .get(requested)
            .map(|text| (requested.to_owned(), text.clone()))
            .ok_or_else(|| format!("include file '{requested}' was not found"))
    }
}
