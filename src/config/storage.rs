use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    pub snapshots: Vec<StorageSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub recorded_at: DateTime<Utc>,
    pub total_bytes: u64,
    pub snapshot_count: u32,
}

impl StorageSnapshot {
    pub fn new(total_bytes: u64, snapshot_count: u32) -> Self {
        Self {
            recorded_at: Utc::now(),
            total_bytes,
            snapshot_count,
        }
    }
}

impl StorageConfig {
    pub fn push_snapshot(&mut self, snap: StorageSnapshot) {
        self.snapshots.push(snap);
        if self.snapshots.len() > 90 {
            self.snapshots.drain(0..self.snapshots.len() - 90);
        }
    }

    pub fn growth_rate_bytes_per_day(&self) -> Option<f64> {
        if self.snapshots.len() < 2 {
            return None;
        }
        let first = &self.snapshots[0];
        let last = &self.snapshots[self.snapshots.len() - 1];
        let days = (last.recorded_at - first.recorded_at).num_seconds() as f64 / 86400.0;
        if days < 0.01 {
            return None;
        }
        Some((last.total_bytes as f64 - first.total_bytes as f64) / days)
    }

    pub fn days_until_budget(&self, budget_bytes: u64, current_bytes: u64) -> Option<f64> {
        let rate = self.growth_rate_bytes_per_day()?;
        if rate <= 0.0 {
            return None;
        }
        let remaining = budget_bytes.saturating_sub(current_bytes) as f64;
        Some(remaining / rate)
    }
}
