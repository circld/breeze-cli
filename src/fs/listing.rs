use crate::error::ExplorerError;
use std::fs;
use std::path::Path;

pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<String>, ExplorerError> {
    let entries = fs::read_dir(path)?;
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        files.push(name);
    }

    files.sort();
    Ok(files)
}
