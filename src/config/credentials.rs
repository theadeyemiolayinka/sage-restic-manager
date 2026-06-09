use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;
use super::app::atomic_write_restricted;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredentialsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub repository_password: String,
}

impl CredentialsConfig {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("credentials.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default = Self::default();
            default.save()?;
            return Ok(default);
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).map_err(AppError::from)?;
        atomic_write_restricted(&path, content.as_bytes())?;
        Ok(())
    }
}
