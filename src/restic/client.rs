use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;


use crate::config::AppConfig;
use crate::error::{AppError, Result};
use super::progress::{BackupProgress, ProgressEvent};
use super::types::{ForgetResult, ResticCheckResult, ResticStats, RestoreTarget, Snapshot};

pub struct ResticClient {
    binary: String,
    repository: String,
    env: Vec<(String, String)>,
}

impl ResticClient {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            binary: config.restic_binary.clone(),
            repository: config.restic_repository_url(),
            env: config.restic_env(),
        }
    }

    fn base_command(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("--repo").arg(&self.repository);
        cmd.arg("--json");
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    fn base_command_no_json(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("--repo").arg(&self.repository);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    pub async fn version(&self) -> Result<String> {
        let output = Command::new(&self.binary)
            .arg("version")
            .output()
            .await
            .map_err(|_| AppError::ResticNotFound)?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("version")
            .output()
            .await
            .is_ok()
    }

    pub async fn init(&self) -> Result<String> {
        let output = self.base_command_no_json()
            .arg("init")
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn snapshots(&self) -> Result<Vec<Snapshot>> {
        let output = self.base_command()
            .arg("snapshots")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let snaps: Vec<Snapshot> = serde_json::from_str(&stdout)?;
            Ok(snaps)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn stats(&self) -> Result<ResticStats> {
        let output = self.base_command()
            .arg("stats")
            .arg("--mode=raw-data")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stats: ResticStats = serde_json::from_str(&stdout)?;
            Ok(stats)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn check(&self, read_data: bool) -> Result<ResticCheckResult> {
        let mut cmd = self.base_command_no_json();
        cmd.arg("check");
        if read_data {
            cmd.arg("--read-data");
        }
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}{}", stdout, stderr);
        Ok(ResticCheckResult {
            ok: output.status.success(),
            output: combined,
        })
    }

    pub async fn forget(&self, policy: &crate::config::RetentionPolicy, dry_run: bool) -> Result<Vec<ForgetResult>> {
        let mut cmd = self.base_command();
        cmd.arg("forget");
        if dry_run {
            cmd.arg("--dry-run");
        }
        if let Some(n) = policy.keep_last {
            cmd.arg("--keep-last").arg(n.to_string());
        }
        if let Some(n) = policy.keep_daily {
            cmd.arg("--keep-daily").arg(n.to_string());
        }
        if let Some(n) = policy.keep_weekly {
            cmd.arg("--keep-weekly").arg(n.to_string());
        }
        if let Some(n) = policy.keep_monthly {
            cmd.arg("--keep-monthly").arg(n.to_string());
        }
        if let Some(n) = policy.keep_yearly {
            cmd.arg("--keep-yearly").arg(n.to_string());
        }
        let output = cmd.output().await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let result: Vec<ForgetResult> = serde_json::from_str(&stdout)
                .unwrap_or_default();
            Ok(result)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn prune(&self) -> Result<String> {
        let output = self.base_command_no_json()
            .arg("prune")
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn restore(&self, target: &RestoreTarget) -> Result<String> {
        let mut cmd = self.base_command_no_json();
        cmd.arg("restore");
        cmd.arg(&target.snapshot_id);
        cmd.arg("--target").arg(&target.target_path);
        if let Some(src) = &target.source_path {
            cmd.arg("--path").arg(src);
        }
        let output = cmd.output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    pub async fn backup_with_progress(
        &self,
        paths: &[PathBuf],
        tags: &[String],
        exclude_patterns: &[String],
        tx: mpsc::Sender<ProgressEvent>,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("--repo").arg(&self.repository);
        cmd.arg("backup");
        cmd.arg("--json");
        for path in paths {
            cmd.arg(path);
        }
        for tag in tags {
            cmd.arg("--tag").arg(tag);
        }
        for pattern in exclude_patterns {
            cmd.arg("--exclude").arg(pattern);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let tx_err = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    let _ = tx_err.send(ProgressEvent::Error(line)).await;
                }
            }
        });

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<BackupProgress>(&line) {
                Ok(progress) => {
                    if progress.is_summary() {
                        let _ = tx.send(ProgressEvent::BackupSummary(progress)).await;
                    } else {
                        let _ = tx.send(ProgressEvent::BackupStatus(progress)).await;
                    }
                }
                Err(_) => {
                    let _ = tx.send(ProgressEvent::RawLine(line)).await;
                }
            }
        }

        let status = child.wait().await?;
        if status.success() {
            let _ = tx.send(ProgressEvent::Finished).await;
            Ok(())
        } else {
            Err(AppError::Restic {
                code: status.code().unwrap_or(-1),
                stderr: "Backup process failed".into(),
            })
        }
    }

    pub async fn ls(&self, snapshot_id: &str) -> Result<Vec<String>> {
        let output = self.base_command()
            .arg("ls")
            .arg(snapshot_id)
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<String> = stdout.lines()
                .filter_map(|line| {
                    serde_json::from_str::<serde_json::Value>(line)
                        .ok()
                        .and_then(|v| v["path"].as_str().map(String::from))
                })
                .collect();
            Ok(lines)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic {
                code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }
}
