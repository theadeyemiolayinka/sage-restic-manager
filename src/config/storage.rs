use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;
use super::app::atomic_write_restricted;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHistory {
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub total_size: u64,
    pub snapshots_count: u32,
}

impl Default for StorageHistory {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl StorageHistory {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("history.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default = Self::default();
            default.save()?;
            return Ok(default);
        }
        let content = std::fs::read_to_string(&path)?;
        let history: Self = toml::from_str(&content)?;
        Ok(history)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).map_err(AppError::from)?;
        atomic_write_restricted(&path, content.as_bytes())?;
        Ok(())
    }

    pub fn push(&mut self, total_size: u64, snapshots_count: u32) {
        let now = Utc::now();
        if let Some(last) = self.entries.last() {
            if last.total_size == total_size && last.snapshots_count == snapshots_count {
                return;
            }
        }
        self.entries.push(HistoryEntry { timestamp: now, total_size, snapshots_count });
        if self.entries.len() > 365 {
            self.entries.remove(0);
        }
    }

    pub fn growth_rate_bytes_per_day(&self) -> Option<f64> {
        if self.entries.len() < 2 {
            return None;
        }
        let first = self.entries.first()?;
        let last = self.entries.last()?;
        let days = (last.timestamp - first.timestamp).num_days() as f64;
        if days <= 0.0 {
            return None;
        }
        let growth = last.total_size.saturating_sub(first.total_size) as f64;
        Some(growth / days)
    }

    pub fn days_until_budget(&self, budget_bytes: u64) -> Option<f64> {
        if budget_bytes == 0 {
            return None;
        }
        let last = self.entries.last()?;
        if last.total_size >= budget_bytes {
            return Some(0.0);
        }
        let rate = self.growth_rate_bytes_per_day()?;
        if rate <= 0.0 {
            return None;
        }
        let remaining = (budget_bytes - last.total_size) as f64;
        Some(remaining / rate)
    }
}
