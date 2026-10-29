use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Platform capability unsupported: {0}")]
    Unsupported(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Internal platform error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
