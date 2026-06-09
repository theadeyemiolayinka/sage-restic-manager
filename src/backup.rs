use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::{AppConfig, SourceState, SourcesConfig};
use crate::error::{AppError, Result};
use crate::restic::{ProgressEvent, ResticClient};

pub async fn run_backup(config: &AppConfig, sources: &mut SourcesConfig) -> Result<()> {
    let selected_paths = sources.selected_paths();

    if selected_paths.is_empty() {
        warn!("No sources selected for backup. Approve and select sources first.");
        return Err(AppError::Config("No sources selected for backup".into()));
    }

    info!("Starting backup of {} sources", selected_paths.len());
    for path in &selected_paths {
        info!("  - {}", path.display());
    }

    let client = ResticClient::new(config);

    if !client.is_available().await {
        return Err(AppError::ResticNotFound);
    }

    let tags = vec!["sage-restic-manager".to_string()];
    let exclude_patterns: Vec<String> = sources.selected_sources()
        .iter()
        .flat_map(|s| s.exclude_patterns.clone())
        .collect();

    let (tx, mut rx) = mpsc::channel(128);

    let client_task = {
        let paths = selected_paths.clone();
        let tags = tags.clone();
        let exclude = exclude_patterns.clone();
        tokio::spawn(async move {
            client.backup_with_progress(&paths, &tags, &exclude, tx).await
        })
    };

    let mut snapshot_id: Option<String> = None;

    while let Some(event) = rx.recv().await {
        match &event {
            ProgressEvent::BackupStatus(p) => {
                info!("Progress: {}", p.display_progress());
            }
            ProgressEvent::BackupSummary(p) => {
                info!("Backup complete: {}", p.display_progress());
                snapshot_id = p.snapshot_id.clone();
            }
            ProgressEvent::Error(e) => {
                warn!("Backup stderr: {}", e);
            }
            ProgressEvent::RawLine(line) => {
                info!("restic: {}", line);
            }
            ProgressEvent::BackupPid(_) => {}
            ProgressEvent::Finished => {
                info!("Backup process finished");
            }
        }
    }

    let result = client_task.await.map_err(|e| AppError::Config(e.to_string()))?;

    let now = chrono::Utc::now();
    match (&result, &snapshot_id) {
        (Ok(_), Some(snap_id)) => {
            for source in sources.sources.iter_mut() {
                if source.state == SourceState::Selected {
                    source.last_backup = Some(now);
                    source.last_snapshot_id = Some(snap_id.clone());
                    source.last_backup_status = Some(crate::config::sources::BackupStatus::Success);
                }
            }
            sources.save()?;
            info!("Snapshot created: {}", snap_id);
        }
        (Err(_), _) | (Ok(_), None) => {
            for source in sources.sources.iter_mut() {
                if source.state == SourceState::Selected {
                    source.last_backup = Some(now);
                    source.last_backup_status = Some(crate::config::sources::BackupStatus::Failed);
                }
            }
            let _ = sources.save();
        }
    }

    result.map(|_| ())
}
