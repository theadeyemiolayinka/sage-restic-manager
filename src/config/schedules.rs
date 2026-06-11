use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;
use super::app::atomic_write_restricted;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesConfig {
    pub schedules: Vec<ScheduleConfig>,
}

impl Default for SchedulesConfig {
    fn default() -> Self {
        Self { schedules: Vec::new() }
    }
}

impl SchedulesConfig {
    pub fn config_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("schedules.toml"))
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
        for sched in &config.schedules {
            if let Some(cal) = &sched.on_calendar {
                validate_on_calendar(cal).map_err(|e| AppError::Config(format!("Invalid on_calendar in schedules.toml: {}", e)))?;
            }
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).map_err(AppError::from)?;
        atomic_write_restricted(&path, content.as_bytes())?;
        Ok(())
    }

    pub fn active_schedule(&self) -> Option<&ScheduleConfig> {
        self.schedules.iter().find(|s| s.enabled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub name: String,
    pub enabled: bool,
    pub frequency: ScheduleFrequency,
    pub on_calendar: Option<String>,
    pub run_after_boot_sec: Option<u64>,
    pub run_on_battery: Option<bool>,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            enabled: false,
            frequency: ScheduleFrequency::TwiceWeekly,
            on_calendar: None,
            run_after_boot_sec: None,
            run_on_battery: None,
        }
    }
}

pub fn validate_on_calendar(value: &str) -> std::result::Result<(), &'static str> {
    if value.is_empty() {
        return Err("on_calendar must not be empty");
    }
    for ch in value.chars() {
        if !ch.is_ascii_alphanumeric()
            && !" *:-,./".contains(ch)
        {
            return Err("on_calendar contains invalid characters");
        }
    }
    Ok(())
}

impl ScheduleConfig {
    pub fn on_calendar_value(&self) -> String {
        if let Some(custom) = &self.on_calendar {
            let safe = custom.replace(['\n', '\r', '[', ']'], "_");
            return safe;
        }
        match self.frequency {
            ScheduleFrequency::Daily => "daily".into(),
            ScheduleFrequency::TwiceWeekly => "Mon,Thu 02:00:00".into(),
            ScheduleFrequency::Weekly => "weekly".into(),
            ScheduleFrequency::Custom => self.on_calendar.clone().unwrap_or_else(|| "daily".into()),
        }
    }

    pub fn systemd_timer_content(&self, _binary_path: &str) -> String {
        let mut timer = format!(
            "[Unit]\nDescription=sage-restic-manager scheduled backup\nRequires=sage-restic-manager.service\n\n[Timer]\nOnCalendar={}\nPersistent=true\nRandomizedDelaySec=1800\n",
            self.on_calendar_value()
        );
        if let Some(secs) = self.run_after_boot_sec {
            timer.push_str(&format!("OnBootSec={}s\n", secs));
        }
        timer.push_str("\n[Install]\nWantedBy=timers.target\n");
        timer
    }

    pub fn systemd_service_content(&self, binary_path: &str) -> crate::error::Result<String> {
        validate_systemd_binary_path(binary_path)?;
        let path_env = std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin:/snap/bin".into());
        let mut service = format!(
            "[Unit]\nDescription=sage-restic-manager backup job\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={} backup --non-interactive\nStandardOutput=journal\nStandardError=journal\nEnvironment=\"PATH={}\"\n",
            binary_path,
            path_env
        );
        if self.run_on_battery == Some(false) {
            service.push_str("ConditionACPower=true\n");
        }
        Ok(service)
    }
}

fn validate_systemd_binary_path(path: &str) -> crate::error::Result<()> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(crate::error::AppError::Config(format!("binary_path must be absolute: {}", path)));
    }
    if path.contains(' ') || path.contains(';') || path.contains('|') || path.contains('&') || path.contains('$') {
        return Err(crate::error::AppError::Config(format!("binary_path contains unsafe characters: {}", path)));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleFrequency {
    Daily,
    TwiceWeekly,
    Weekly,
    Custom,
}

impl std::fmt::Display for ScheduleFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleFrequency::Daily => write!(f, "Daily"),
            ScheduleFrequency::TwiceWeekly => write!(f, "Twice Weekly"),
            ScheduleFrequency::Weekly => write!(f, "Weekly"),
            ScheduleFrequency::Custom => write!(f, "Custom"),
        }
    }
}
