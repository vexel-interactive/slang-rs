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
                return Ok((path.to_string_lossy().into_owned(), text));
            }
        }
        Err(format!("include file '{requested}' was not found"))
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
