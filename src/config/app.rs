use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub repository: RepositoryConfig,
    pub retention: RetentionPolicy,
    pub budget: StorageBudget,
    pub restic_binary: String,
    pub update_channel: UpdateChannel,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            repository: RepositoryConfig::default(),
            retention: RetentionPolicy::default(),
            budget: StorageBudget::default(),
            restic_binary: "restic".into(),
            update_channel: UpdateChannel::Stable,
            log_level: "info".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub backend: RepositoryBackend,
    pub url: String,
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub path: String,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            backend: RepositoryBackend::S3,
            url: String::new(),
            bucket: String::new(),
            region: "auto".into(),
            endpoint: None,
            path: "/backups".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryBackend {
    S3,
    B2,
    R2,
    MinIO,
}

impl std::fmt::Display for RepositoryBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryBackend::S3 => write!(f, "AWS S3"),
            RepositoryBackend::B2 => write!(f, "Backblaze B2"),
            RepositoryBackend::R2 => write!(f, "Cloudflare R2"),
            RepositoryBackend::MinIO => write!(f, "MinIO"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_last: Option<u32>,
    pub keep_daily: Option<u32>,
    pub keep_weekly: Option<u32>,
    pub keep_monthly: Option<u32>,
    pub keep_yearly: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_last: Some(5),
            keep_daily: Some(7),
            keep_weekly: Some(4),
            keep_monthly: Some(12),
            keep_yearly: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBudget {
    pub total_bytes: u64,
    pub warning_bytes: u64,
    pub critical_bytes: u64,
    pub enabled: bool,
}

impl Default for StorageBudget {
    fn default() -> Self {
        Self {
            total_bytes: 8 * 1024 * 1024 * 1024,
            warning_bytes: 6 * 1024 * 1024 * 1024,
            critical_bytes: 7 * 1024 * 1024 * 1024 + 512 * 1024 * 1024,
            enabled: true,
        }
    }
}

impl StorageBudget {
    pub fn budget_gib(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn warning_gib(&self) -> f64 {
        self.warning_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn critical_gib(&self) -> f64 {
        self.critical_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

impl std::fmt::Display for UpdateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateChannel::Stable => write!(f, "stable"),
            UpdateChannel::Beta => write!(f, "beta"),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("config.toml"))
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

    pub fn backend_env(&self, creds: &super::credentials::CredentialsConfig) -> Vec<(String, String)> {
        let mut env = Vec::new();
        match self.repository.backend {
            RepositoryBackend::B2 => {
                env.push(("B2_ACCOUNT_ID".into(), creds.access_key_id.clone()));
                env.push(("B2_ACCOUNT_KEY".into(), creds.secret_access_key.clone()));
            }
            _ => {
                env.push(("AWS_ACCESS_KEY_ID".into(), creds.access_key_id.clone()));
                env.push(("AWS_SECRET_ACCESS_KEY".into(), creds.secret_access_key.clone()));
                if let Some(ep) = &self.repository.endpoint {
                    env.push(("AWS_ENDPOINT_URL_S3".into(), ep.clone()));
                }
            }
        }
        env
    }

    pub fn restic_repository_url(&self) -> String {
        match self.repository.backend {
            RepositoryBackend::R2 | RepositoryBackend::S3 | RepositoryBackend::MinIO => {
                format!("s3:{}/{}{}", self.repository.url, self.repository.bucket, self.repository.path)
            }
            RepositoryBackend::B2 => {
                format!("b2:{}{}", self.repository.bucket, self.repository.path)
            }
        }
    }
}

pub(crate) fn atomic_write_restricted(path: &std::path::Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().ok_or_else(|| AppError::Config("Config path has no parent directory".into()))?;
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::write(tmp.path(), data)?;
    tmp.persist(path).map_err(|e| AppError::Io(e.error))?;
    Ok(())
}
