use crate::error::ExplorerError;
use crate::fs::list_directory;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Explorer {
    pub current_dir: PathBuf,
}

impl Explorer {
    pub fn new(directory: PathBuf) -> Result<Self, ExplorerError> {
        if !directory.exists() {
            return Err(ExplorerError::InvalidDirectory(
                directory.to_string_lossy().to_string(),
            ));
        }

        Ok(Explorer {
            current_dir: directory
                .canonicalize()
                .expect("Failed to get absolute path"),
        })
    }

    pub fn ls(&self) -> Result<Vec<String>, ExplorerError> {
        list_directory(&self.current_dir)
    }

    pub fn cd(&mut self, directory: PathBuf) -> Result<Vec<String>, ExplorerError> {
        self.current_dir = directory
            .canonicalize()
            .expect("Failed to get absolute path");
        self.ls()
    }
}
