use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{AppError, Result};
use super::config_dir;

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
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).map_err(AppError::from)?;
        std::fs::write(path, content)?;
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
    pub run_on_battery: bool,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            enabled: false,
            frequency: ScheduleFrequency::TwiceWeekly,
            on_calendar: None,
            run_after_boot_sec: Some(300),
            run_on_battery: true,
        }
    }
}

impl ScheduleConfig {
    pub fn on_calendar_value(&self) -> String {
        if let Some(custom) = &self.on_calendar {
            return custom.clone();
        }
        match self.frequency {
            ScheduleFrequency::Daily => "daily".into(),
            ScheduleFrequency::TwiceWeekly => "Mon,Thu 02:00:00".into(),
            ScheduleFrequency::Weekly => "weekly".into(),
            ScheduleFrequency::Custom => self.on_calendar.clone().unwrap_or_else(|| "daily".into()),
        }
    }

    pub fn systemd_timer_content(&self, binary_path: &str) -> String {
        format!(
            "[Unit]\nDescription=sage-restic-manager scheduled backup\nRequires=sage-restic-manager.service\n\n[Timer]\nOnCalendar={}\nPersistent=true\nRandomizedDelaySec=1800\n\n[Install]\nWantedBy=timers.target\n",
            self.on_calendar_value()
        )
    }

    pub fn systemd_service_content(&self, binary_path: &str) -> String {
        format!(
            "[Unit]\nDescription=sage-restic-manager backup job\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={} backup --non-interactive\nStandardOutput=journal\nStandardError=journal\nUser=root\n",
            binary_path
        )
    }
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
