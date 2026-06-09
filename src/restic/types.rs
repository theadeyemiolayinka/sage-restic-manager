use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub short_id: String,
    pub time: DateTime<Utc>,
    pub hostname: String,
    pub username: String,
    pub paths: Vec<String>,
    pub tags: Option<Vec<String>>,
    pub parent: Option<String>,
}

impl Snapshot {
    pub fn display_paths(&self) -> String {
        self.paths.join(", ")
    }

    pub fn age_description(&self) -> String {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.time);
        if diff.num_days() > 0 {
            format!("{} days ago", diff.num_days())
        } else if diff.num_hours() > 0 {
            format!("{} hours ago", diff.num_hours())
        } else {
            format!("{} minutes ago", diff.num_minutes())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticStats {
    pub total_size: u64,
    pub total_file_count: u64,
    pub total_blob_count: Option<u64>,
    pub snapshots_count: Option<u32>,
}

impl ResticStats {
    pub fn display_size(&self) -> String {
        bytesize::ByteSize(self.total_size).to_string_as(true)
    }
}

#[derive(Debug, Clone)]
pub struct RestoreTarget {
    pub snapshot_id: String,
    pub source_path: Option<PathBuf>,
    pub target_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetResult {
    pub keep: Vec<SnapshotRef>,
    pub remove: Vec<SnapshotRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRef {
    pub id: String,
    pub short_id: String,
    pub time: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticCheckResult {
    pub ok: bool,
    pub output: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResticVersion {
    pub version: String,
}

impl ResticVersion {
    pub fn parse(output: &str) -> Option<Self> {
        let trimmed = output.trim();
        let version_part = trimmed.split_whitespace().nth(1)?;
        Some(Self { version: version_part.into() })
    }
}

