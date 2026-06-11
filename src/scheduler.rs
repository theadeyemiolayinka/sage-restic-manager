use tokio::process::Command;
use tracing::info;

use crate::config::ScheduleConfig;
use crate::error::{AppError, Result};

const SERVICE_NAME: &str = "sage-restic-manager";
fn systemd_user_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".config/systemd/user")
}

pub struct SystemdScheduler;

impl SystemdScheduler {
    pub async fn install(schedule: &ScheduleConfig, binary_path: &str) -> Result<()> {
        let service_content = schedule.systemd_service_content(binary_path)?;
        let timer_content = schedule.systemd_timer_content(binary_path);

        let user_dir = systemd_user_dir();
        tokio::fs::create_dir_all(&user_dir)
            .await
            .map_err(|e| AppError::PermissionDenied(format!("Cannot create systemd user dir: {}", e)))?;

        let service_path = user_dir.join(format!("{}.service", SERVICE_NAME));
        let timer_path = user_dir.join(format!("{}.timer", SERVICE_NAME));

        tokio::fs::write(&service_path, service_content)
            .await
            .map_err(|e| AppError::PermissionDenied(format!("Cannot write service file: {}", e)))?;

        tokio::fs::write(&timer_path, timer_content)
            .await
            .map_err(|e| AppError::PermissionDenied(format!("Cannot write timer file: {}", e)))?;

        systemctl_run(&["daemon-reload"]).await?;
        info!("Systemd units installed: {} and {}", service_path.display(), timer_path.display());
        Ok(())
    }

    pub async fn enable() -> Result<()> {
        systemctl_run(&["enable", "--now", &format!("{}.timer", SERVICE_NAME)]).await?;
        info!("Timer enabled and started");
        Ok(())
    }

    pub async fn disable() -> Result<()> {
        systemctl_run(&["disable", "--now", &format!("{}.timer", SERVICE_NAME)]).await?;
        info!("Timer disabled");
        Ok(())
    }

    pub async fn status() -> Result<String> {
        let output = Command::new("systemctl")
            .arg("--user")
            .arg("status")
            .arg(format!("{}.timer", SERVICE_NAME))
            .output()
            .await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn is_active() -> bool {
        Command::new("systemctl")
            .arg("--user")
            .arg("is-active")
            .arg("--quiet")
            .arg(format!("{}.timer", SERVICE_NAME))
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub async fn next_trigger_time() -> Option<String> {
        let output = Command::new("systemctl")
            .arg("--user")
            .arg("show")
            .arg(format!("{}.timer", SERVICE_NAME))
            .arg("--property=NextElapseUSecRealtime")
            .output()
            .await
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value = stdout.trim().strip_prefix("NextElapseUSecRealtime=")?.trim();
        if value.is_empty() || value == "0" || value == "infinity" {
            return None;
        }
        Some(value.to_string())
    }

}

async fn systemctl_run(args: &[&str]) -> Result<()> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .await
        .map_err(|e| AppError::Config(format!("systemctl not found or failed to start: {}", e)))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let code = output.status.code().unwrap_or(-1);
        if code == 1 && (stderr.contains("Access denied") || stderr.contains("not permitted")) {
            Err(AppError::PermissionDenied(format!("systemctl: {}", stderr)))
        } else {
            Err(AppError::Config(format!("systemctl --user {} failed (exit {}): {}", args.join(" "), code, stderr)))
        }
    }
}
