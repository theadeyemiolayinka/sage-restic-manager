pub mod app;
pub mod sources;
pub mod schedules;
pub mod storage;

pub use app::{AppConfig, RetentionPolicy};
pub use sources::{BackupSource, ContainerOrigin, SourceKind, SourceState, SourcesConfig};
pub use schedules::{ScheduleConfig, ScheduleFrequency, SchedulesConfig};

use crate::error::{AppError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "sage-restic-manager")
        .ok_or_else(|| AppError::Config("Cannot determine config directory".into()))?;
    let path = dirs.config_dir().to_path_buf();
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn log_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "sage-restic-manager")
        .ok_or_else(|| AppError::Config("Cannot determine data directory".into()))?;
    let path = dirs.data_local_dir().join("logs");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
