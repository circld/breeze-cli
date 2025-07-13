use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExplorerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid directory: {0}")]
    InvalidDirectory(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
