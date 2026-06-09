use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Restic error (exit {code}): {stderr}")]
    Restic { code: i32, stderr: String },

    #[error("Restic not found in PATH")]
    ResticNotFound,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Update error: {0}")]
    Update(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Signature mismatch: expected {expected}, got {actual}")]
    SignatureMismatch { expected: String, actual: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Semver parse error: {0}")]
    Semver(#[from] semver::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::Serialization(e.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serialization(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Config(e.to_string())
    }
}
