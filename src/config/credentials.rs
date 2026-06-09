use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;
use super::app::atomic_write_restricted;

const KEYRING_SERVICE: &str = "sage-restic-manager";
const KEYRING_USER: &str = "credentials";

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CredentialsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub repository_password: String,
}

impl std::fmt::Debug for CredentialsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialsConfig")
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field("repository_password", &"<redacted>")
            .finish()
    }
}

impl CredentialsConfig {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("credentials.toml"))
    }

    pub fn load() -> Result<Self> {
        match Self::load_from_keyring() {
            Ok(creds) => return Ok(creds),
            Err(e) => {
                tracing::warn!("Keyring credential load failed ({}); falling back to plaintext file. Consider checking your OS secret service.", e);
            }
        }
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
        match self.save_to_keyring() {
            Ok(()) => {
                let _ = Self::remove_file_fallback();
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Keyring credential save failed ({}); falling back to plaintext file. Consider checking your OS secret service.", e);
            }
        }
        self.save_to_file()
    }

    fn load_from_keyring() -> Result<Self> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| AppError::Config(format!("keyring entry creation failed: {}", e)))?;
        let json = entry.get_password()
            .map_err(|e| AppError::Config(format!("keyring get_password failed: {}", e)))?;
        let creds: Self = serde_json::from_str(&json)
            .map_err(|e| AppError::Config(format!("keyring credential parse failed: {}", e)))?;
        Ok(creds)
    }

    fn save_to_keyring(&self) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| AppError::Config(format!("keyring entry creation failed: {}", e)))?;
        let json = serde_json::to_string(self)
            .map_err(|e| AppError::Config(format!("credential serialization failed: {}", e)))?;
        entry.set_password(&json)
            .map_err(|e| AppError::Config(format!("keyring set_password failed: {}", e)))?;
        Ok(())
    }

    fn save_to_file(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).map_err(AppError::from)?;
        atomic_write_restricted(&path, content.as_bytes())?;
        Ok(())
    }

    fn remove_file_fallback() -> Result<()> {
        let path = Self::config_path()?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(AppError::from)?;
        }
        Ok(())
    }
}
