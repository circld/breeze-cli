use crate::error::ExplorerError;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<String>, ExplorerError> {
    let entries = fs::read_dir(path)?;
    let mut files = Vec::new();

    for entry in entries {
        let path: PathBuf = entry?.path();
        let abs_path = fs::canonicalize(path)?;
        files.push(abs_path.to_string_lossy().to_string());
    }

    files.sort();
    Ok(files)
}
