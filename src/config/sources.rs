use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::error::{AppError, Result};
use super::config_dir;
use super::app::atomic_write_restricted;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    pub sources: Vec<BackupSource>,
    pub docker_volumes_path: Option<PathBuf>,
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            docker_volumes_path: None,
        }
    }
}

impl SourcesConfig {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("sources.toml"))
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

    pub fn effective_docker_path(&self) -> PathBuf {
        self.docker_volumes_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("/var/lib/docker/volumes"))
    }

    pub fn upsert(&mut self, source: BackupSource) {
        if let Some(existing) = self.sources.iter_mut().find(|s| s.path == source.path) {
            existing.size_bytes = source.size_bytes;
        } else {
            self.sources.push(source);
        }
    }

    pub fn upsert_child(&mut self, source: BackupSource) {
        if !self.sources.iter().any(|s| s.path == source.path) {
            self.sources.push(source);
        } else {
            if let Some(existing) = self.sources.iter_mut().find(|s| s.path == source.path) {
                existing.size_bytes = source.size_bytes;
            }
        }
    }

    pub fn selected_sources(&self) -> Vec<&BackupSource> {
        self.sources.iter().filter(|s| {
            s.state == SourceState::Selected && !matches!(s.kind, SourceKind::Container { .. })
        }).collect()
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_sources().into_iter().map(|s| s.path.clone()).collect()
    }

    pub fn total_selected_bytes(&self) -> u64 {
        self.selected_sources().iter().map(|s| s.size_bytes.unwrap_or(0)).sum()
    }

    pub fn find_by_path_mut(&mut self, path: &PathBuf) -> Option<&mut BackupSource> {
        self.sources.iter_mut().find(|s| &s.path == path)
    }

    pub fn children_of(&self, container_path: &PathBuf) -> Vec<&BackupSource> {
        self.sources.iter().filter(|s| {
            matches!(&s.kind, SourceKind::FlatPath { parent_container: Some(p) } if p == container_path)
        }).collect()
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSource {
    pub id: Uuid,
    pub path: PathBuf,
    pub label: String,
    pub kind: SourceKind,
    pub state: SourceState,
    pub size_bytes: Option<u64>,
    pub first_discovered: DateTime<Utc>,
    pub last_backup: Option<DateTime<Utc>>,
    pub last_backup_status: Option<BackupStatus>,
    pub last_snapshot_id: Option<String>,
    pub exclude_patterns: Vec<String>,
}

impl BackupSource {
    pub fn new_unapproved(path: PathBuf, label: String, kind: SourceKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
            label,
            kind,
            state: SourceState::Unapproved,
            size_bytes: None,
            first_discovered: Utc::now(),
            last_backup: None,
            last_backup_status: None,
            last_snapshot_id: None,
            exclude_patterns: Vec::new(),
        }
    }

    pub fn new_container(path: PathBuf, label: String, origin: ContainerOrigin) -> Self {
        Self::new_unapproved(path, label, SourceKind::Container { origin })
    }

    pub fn new_flat_child(path: PathBuf, label: String, parent: PathBuf) -> Self {
        Self::new_unapproved(path, label, SourceKind::FlatPath { parent_container: Some(parent) })
    }

    pub fn new_flat_standalone(path: PathBuf, label: String) -> Self {
        Self::new_unapproved(path, label, SourceKind::FlatPath { parent_container: None })
    }

    pub fn display_size(&self) -> String {
        match self.size_bytes {
            Some(b) => bytesize::ByteSize(b).to_string_as(true),
            None => "?".into(),
        }
    }

    pub fn is_container(&self) -> bool {
        matches!(self.kind, SourceKind::Container { .. })
    }

    pub fn kind_label(&self) -> &'static str {
        match &self.kind {
            SourceKind::Container { origin: ContainerOrigin::DockerVolumes } => "docker-dir",
            SourceKind::Container { origin: ContainerOrigin::CustomDirectory } => "dir",
            SourceKind::FlatPath { parent_container: Some(_) } => "child",
            SourceKind::FlatPath { parent_container: None } => "flat",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceKind {
    FlatPath {
        parent_container: Option<PathBuf>,
    },
    Container {
        origin: ContainerOrigin,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerOrigin {
    DockerVolumes,
    CustomDirectory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceState {
    Unapproved,
    Selected,
    Ignored,
}

impl std::fmt::Display for SourceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceState::Unapproved => write!(f, "unapproved"),
            SourceState::Selected => write!(f, "selected"),
            SourceState::Ignored => write!(f, "ignored"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    Success,
    Failed,
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupStatus::Success => write!(f, "success"),
            BackupStatus::Failed => write!(f, "failed"),
        }
    }
}
