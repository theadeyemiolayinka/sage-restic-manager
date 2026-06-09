pub mod app;
pub mod sources;
pub mod schedules;
pub mod credentials;

pub use app::{AppConfig, RetentionPolicy};
pub use sources::{BackupSource, ContainerOrigin, SourceKind, SourceState, SourcesConfig};
pub use schedules::{ScheduleConfig, ScheduleFrequency, SchedulesConfig};
pub use credentials::CredentialsConfig;

use crate::error::{AppError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::OnceLock;

static CONFIG_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

pub fn set_config_dir_override(path: PathBuf) {
    let _ = CONFIG_DIR_OVERRIDE.set(path);
}

pub fn config_dir() -> Result<PathBuf> {
    if let Some(override_path) = CONFIG_DIR_OVERRIDE.get() {
        create_dir_restricted(override_path)?;
        return Ok(override_path.clone());
    }
    let dirs = ProjectDirs::from("", "", "sage-restic-manager")
        .ok_or_else(|| AppError::Config("Cannot determine config directory".into()))?;
    let path = dirs.config_dir().to_path_buf();
    create_dir_restricted(&path)?;
    Ok(path)
}

pub fn log_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "sage-restic-manager")
        .ok_or_else(|| AppError::Config("Cannot determine data directory".into()))?;
    let path = dirs.data_local_dir().join("logs");
    create_dir_restricted(&path)?;
    Ok(path)
}

fn create_dir_restricted(path: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
