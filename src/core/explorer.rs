use crate::error::ExplorerError;
use crate::fs::list_directory;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Explorer {
    current_dir: PathBuf,
}

impl Explorer {
    pub fn new(directory: PathBuf) -> Result<Self, ExplorerError> {
        if !directory.exists() {
            return Err(ExplorerError::InvalidDirectory(
                directory.to_string_lossy().to_string(),
            ));
        }

        Ok(Explorer {
            current_dir: directory,
        })
    }

    pub fn run(&self) -> Result<(), ExplorerError> {
        let files = list_directory(&self.current_dir)?;

        // Simple output for Phase 1
        println!("{}", self.current_dir.display());
        for file in files {
            println!("{}", file);
        }

        Ok(())
    }
}
