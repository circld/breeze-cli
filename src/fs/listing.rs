use crate::error::ExplorerError;
use std::fs::{self, DirEntry};
use std::path::Path;

pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>, ExplorerError> {
    let entries = fs::read_dir(path)?;
    let mut files = Vec::new();

    for entry in entries {
        files.push(entry?);
    }

    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}
