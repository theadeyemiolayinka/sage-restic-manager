use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgress {
    pub message_type: String,
    pub files_new: Option<u64>,
    pub files_changed: Option<u64>,
    pub files_unmodified: Option<u64>,
    pub dirs_new: Option<u64>,
    pub dirs_changed: Option<u64>,
    pub dirs_unmodified: Option<u64>,
    pub data_added: Option<u64>,
    pub total_files_processed: Option<u64>,
    pub total_bytes_processed: Option<u64>,
    pub total_duration: Option<f64>,
    pub snapshot_id: Option<String>,
    pub percent_done: Option<f64>,
    pub total_files: Option<u64>,
    pub total_bytes: Option<u64>,
    pub current_files: Option<Vec<String>>,
    pub error_count: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    BackupStatus(BackupProgress),
    BackupSummary(BackupProgress),
    BackupPid(u32),
    RawLine(String),
    Error(String),
    Finished,
}

impl BackupProgress {
    pub fn is_summary(&self) -> bool {
        self.message_type == "summary"
    }

    pub fn percent(&self) -> f64 {
        self.percent_done.unwrap_or(0.0).min(1.0)
    }

    pub fn display_progress(&self) -> String {
        let pct = (self.percent() * 100.0) as u64;
        let added = bytesize::ByteSize(self.data_added.unwrap_or(0)).to_string_as(true);
        let processed = bytesize::ByteSize(self.total_bytes_processed.unwrap_or(0)).to_string_as(true);
        format!("{}% - {} added - {} processed", pct, added, processed)
    }
}
