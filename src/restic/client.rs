use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::config::{AppConfig, CredentialsConfig};
use crate::error::{AppError, Result};
use super::progress::{BackupProgress, ProgressEvent};
use super::types::{ForgetResult, ResticCheckResult, ResticStats, RestoreTarget, Snapshot};

pub struct ResticClient {
    binary: String,
    repository: String,
    backend_env: Vec<(String, String)>,
    password: String,
}

fn validate_binary(binary: &str) -> Result<()> {
    let forbidden: &[char] = &[';', '&', '|', '`', '$', '(', ')', '<', '>', '\n', '\r', '\0'];
    if binary.chars().any(|c| forbidden.contains(&c)) {
        return Err(AppError::Config(format!("restic_binary contains forbidden characters: {}", binary)));
    }
    if binary.is_empty() {
        return Err(AppError::Config("restic_binary must not be empty".into()));
    }
    Ok(())
}

impl ResticClient {
    pub fn new(config: &AppConfig) -> Self {
        let creds = CredentialsConfig::load().unwrap_or_default();
        Self::new_with_creds(config, &creds)
    }

    pub fn new_with_creds(config: &AppConfig, creds: &CredentialsConfig) -> Self {
        let backend_env = config.backend_env(creds);
        let password = creds.repository_password.clone();
        Self {
            binary: config.restic_binary.clone(),
            repository: config.restic_repository_url(),
            backend_env,
            password,
        }
    }

    fn write_password_file(&self) -> Result<tempfile::NamedTempFile> {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new()?;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
        std::fs::write(tmp.path(), self.password.as_bytes())?;
        Ok(tmp)
    }

    fn write_backend_cred_file(&self) -> Result<Option<tempfile::NamedTempFile>> {
        use std::os::unix::fs::PermissionsExt;
        let has_b2 = self.backend_env.iter().any(|(k, _)| k == "B2_ACCOUNT_ID");
        let has_s3 = self.backend_env.iter().any(|(k, _)| k == "AWS_ACCESS_KEY_ID");
        if has_s3 {
            let key_id = self.backend_env.iter().find(|(k, _)| k == "AWS_ACCESS_KEY_ID").map(|(_, v)| v.as_str()).unwrap_or("");
            let secret = self.backend_env.iter().find(|(k, _)| k == "AWS_SECRET_ACCESS_KEY").map(|(_, v)| v.as_str()).unwrap_or("");
            let content = format!("[default]\naws_access_key_id={}\naws_secret_access_key={}\n", key_id, secret);
            let tmp = tempfile::NamedTempFile::new()?;
            std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
            std::fs::write(tmp.path(), content.as_bytes())?;
            return Ok(Some(tmp));
        }
        if has_b2 {
            let account_id = self.backend_env.iter().find(|(k, _)| k == "B2_ACCOUNT_ID").map(|(_, v)| v.as_str()).unwrap_or("");
            let account_key = self.backend_env.iter().find(|(k, _)| k == "B2_ACCOUNT_KEY").map(|(_, v)| v.as_str()).unwrap_or("");
            let content = format!("{}:{}", account_id, account_key);
            let tmp = tempfile::NamedTempFile::new()?;
            std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
            std::fs::write(tmp.path(), content.as_bytes())?;
            return Ok(Some(tmp));
        }
        Ok(None)
    }

    fn apply_backend_cred_env(cmd: &mut Command, backend_env: &[(String, String)], cred_file: &Option<tempfile::NamedTempFile>) {
        let has_b2 = backend_env.iter().any(|(k, _)| k == "B2_ACCOUNT_ID");
        let has_s3 = backend_env.iter().any(|(k, _)| k == "AWS_ACCESS_KEY_ID");
        if has_s3 {
            if let Some(f) = cred_file {
                cmd.env("AWS_SHARED_CREDENTIALS_FILE", f.path());
            }
            if let Some((_, ep)) = backend_env.iter().find(|(k, _)| k == "AWS_ENDPOINT_URL_S3") {
                cmd.env("AWS_ENDPOINT_URL_S3", ep);
            }
        } else if has_b2 {
            if let Some(f) = cred_file {
                cmd.env("B2_ACCOUNT_CREDENTIALS_FILE", f.path());
            }
        }
    }

    fn base_command_with_pass<'a>(
        &self,
        pass_file: &'a tempfile::NamedTempFile,
        cred_file: &'a Option<tempfile::NamedTempFile>,
    ) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.env_clear();
        cmd.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into()));
        cmd.arg("--repo").arg(&self.repository);
        cmd.arg("--password-file").arg(pass_file.path());
        cmd.arg("--json");
        Self::apply_backend_cred_env(&mut cmd, &self.backend_env, cred_file);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    fn base_command_no_json_with_pass<'a>(
        &self,
        pass_file: &'a tempfile::NamedTempFile,
        cred_file: &'a Option<tempfile::NamedTempFile>,
    ) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.env_clear();
        cmd.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into()));
        cmd.arg("--repo").arg(&self.repository);
        cmd.arg("--password-file").arg(pass_file.path());
        Self::apply_backend_cred_env(&mut cmd, &self.backend_env, cred_file);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    fn base_command(&self) -> Result<(Command, tempfile::NamedTempFile, Option<tempfile::NamedTempFile>)> {
        validate_binary(&self.binary)?;
        let pass_file = self.write_password_file()?;
        let cred_file = self.write_backend_cred_file()?;
        let cmd = self.base_command_with_pass(&pass_file, &cred_file);
        Ok((cmd, pass_file, cred_file))
    }

    fn base_command_no_json(&self) -> Result<(Command, tempfile::NamedTempFile, Option<tempfile::NamedTempFile>)> {
        validate_binary(&self.binary)?;
        let pass_file = self.write_password_file()?;
        let cred_file = self.write_backend_cred_file()?;
        let cmd = self.base_command_no_json_with_pass(&pass_file, &cred_file);
        Ok((cmd, pass_file, cred_file))
    }

    pub async fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("version")
            .output()
            .await
            .is_ok()
    }

    pub async fn init(&self) -> Result<String> {
        let (mut cmd, _pass, _creds) = self.base_command_no_json()?;
        let output = cmd.arg("init").output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn snapshots(&self) -> Result<Vec<Snapshot>> {
        let (mut cmd, _pass, _creds) = self.base_command()?;
        let output = cmd.arg("snapshots").output().await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let snaps: Vec<Snapshot> = serde_json::from_str(&stdout)?;
            Ok(snaps)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn stats(&self) -> Result<ResticStats> {
        let (mut cmd, _pass, _creds) = self.base_command()?;
        let output = cmd.arg("stats").arg("--mode=raw-data").output().await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stats: ResticStats = serde_json::from_str(&stdout)?;
            Ok(stats)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn check(&self, read_data: bool) -> Result<ResticCheckResult> {
        let (mut cmd, _pass, _creds) = self.base_command_no_json()?;
        cmd.arg("check");
        if read_data { cmd.arg("--read-data"); }
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ResticCheckResult {
            ok: output.status.success(),
            output: format!("{}{}", stdout, stderr),
        })
    }

    pub async fn forget(&self, policy: &crate::config::RetentionPolicy, dry_run: bool) -> Result<Vec<ForgetResult>> {
        let (mut cmd, _pass, _creds) = self.base_command()?;
        cmd.arg("forget");
        if dry_run { cmd.arg("--dry-run"); }
        if let Some(n) = policy.keep_last   { cmd.arg("--keep-last").arg(n.to_string()); }
        if let Some(n) = policy.keep_daily  { cmd.arg("--keep-daily").arg(n.to_string()); }
        if let Some(n) = policy.keep_weekly { cmd.arg("--keep-weekly").arg(n.to_string()); }
        if let Some(n) = policy.keep_monthly { cmd.arg("--keep-monthly").arg(n.to_string()); }
        if let Some(n) = policy.keep_yearly { cmd.arg("--keep-yearly").arg(n.to_string()); }
        let output = cmd.output().await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let result: Vec<ForgetResult> = serde_json::from_str(&stdout)
                .map_err(|e| AppError::Config(format!("Failed to parse forget output: {}", e)))?;
            Ok(result)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn prune(&self) -> Result<String> {
        let (mut cmd, _pass, _creds) = self.base_command_no_json()?;
        let output = cmd.arg("prune").output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn restore(&self, target: &RestoreTarget) -> Result<String> {
        let (mut cmd, _pass, _creds) = self.base_command_no_json()?;
        cmd.arg("restore").arg(&target.snapshot_id).arg("--target").arg(&target.target_path);
        if let Some(src) = &target.source_path { cmd.arg("--path").arg(src); }
        let output = cmd.output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::Restic { code: output.status.code().unwrap_or(-1), stderr })
        }
    }

    pub async fn backup_with_progress(
        &self,
        paths: &[PathBuf],
        tags: &[String],
        exclude_patterns: &[String],
        tx: mpsc::Sender<ProgressEvent>,
    ) -> Result<u32> {
        validate_binary(&self.binary)?;
        let pass_file = self.write_password_file()?;
        let cred_file = self.write_backend_cred_file()?;
        let mut cmd = self.base_command_no_json_with_pass(&pass_file, &cred_file);
        cmd.arg("backup").arg("--json");
        for path in paths { cmd.arg(path); }
        for tag in tags { cmd.arg("--tag").arg(tag); }
        for pattern in exclude_patterns { cmd.arg("--exclude").arg(pattern); }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let child_id = child.id().unwrap_or(0);
        let _ = tx.send(ProgressEvent::BackupPid(child_id)).await;

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
        drop(pass_file);
        drop(cred_file);
        if status.success() {
            let _ = tx.send(ProgressEvent::Finished).await;
            Ok(child_id)
        } else {
            Err(AppError::Restic {
                code: status.code().unwrap_or(-1),
                stderr: "Backup process failed".into(),
            })
        }
    }
}
