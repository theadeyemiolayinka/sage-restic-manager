use tokio::process::Command;
use tracing::info;

use crate::config::ScheduleConfig;
use crate::error::{AppError, Result};

const SERVICE_NAME: &str = "sage-restic-manager";
const SYSTEMD_USER_DIR: &str = "/etc/systemd/system";

pub struct SystemdScheduler;

impl SystemdScheduler {
    pub async fn install(schedule: &ScheduleConfig, binary_path: &str) -> Result<()> {
        let service_content = schedule.systemd_service_content(binary_path);
        let timer_content = schedule.systemd_timer_content(binary_path);

        let service_path = format!("{}/{}.service", SYSTEMD_USER_DIR, SERVICE_NAME);
        let timer_path = format!("{}/{}.timer", SYSTEMD_USER_DIR, SERVICE_NAME);

        tokio::fs::write(&service_path, service_content)
            .await
            .map_err(|e| AppError::PermissionDenied(format!("Cannot write service file: {}", e)))?;

        tokio::fs::write(&timer_path, timer_content)
            .await
            .map_err(|e| AppError::PermissionDenied(format!("Cannot write timer file: {}", e)))?;

        systemctl_run(&["daemon-reload"]).await?;
        info!("Systemd units installed: {} and {}", service_path, timer_path);
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
            .arg("status")
            .arg(format!("{}.timer", SERVICE_NAME))
            .output()
            .await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn is_active() -> bool {
        Command::new("systemctl")
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
            .arg("list-timers")
            .arg(format!("{}.timer", SERVICE_NAME))
            .arg("--no-legend")
            .output()
            .await
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            Some(format!("{} {}", parts[0], parts[1]))
        } else {
            None
        }
    }

    pub fn generate_service_content(binary_path: &str) -> String {
        format!(
            "[Unit]\nDescription=sage-restic-manager backup job\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={} backup --non-interactive\nStandardOutput=journal\nStandardError=journal\nUser=root\n",
            binary_path
        )
    }

    pub fn generate_timer_content(on_calendar: &str) -> String {
        format!(
            "[Unit]\nDescription=sage-restic-manager scheduled backup\nRequires={}.service\n\n[Timer]\nOnCalendar={}\nPersistent=true\nRandomizedDelaySec=1800\n\n[Install]\nWantedBy=timers.target\n",
            SERVICE_NAME,
            on_calendar
        )
    }
}

async fn systemctl_run(args: &[&str]) -> Result<()> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(AppError::PermissionDenied(format!("systemctl failed: {}", stderr)))
    }
}
