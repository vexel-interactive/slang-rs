use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub fn resolved_path(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let filename = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "output path has no filename")
    })?;
    Ok(parent.canonicalize()?.join(filename))
}

pub fn validate_destination(path: &Path, sources: &slang_compiler::SourceMap) -> io::Result<()> {
    let destination = resolved_path(path)?;
    for source in sources.files() {
        if Path::new(&source.name)
            .canonicalize()
            .is_ok_and(|p| p == destination)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output path would overwrite an input or included source file",
            ));
        }
    }
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let filename = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "output path has no filename")
    })?;
    let mut temporary_name = filename.to_os_string();
    temporary_name.push(format!(".slang-rs-{}.tmp", std::process::id()));
    let temporary = parent.join(temporary_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
