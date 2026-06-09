use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::config::{AppConfig, BackupSource, CredentialsConfig, SchedulesConfig, SourceKind, SourcesConfig};
use crate::restic::{ResticStats, Snapshot};
use crate::tui::event::BackgroundEvent;

pub const MAX_LOG_LINES: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    Sources,
    Repository,
    Snapshots,
    Restore,
    Scheduler,
    Logs,
    Settings,
}

impl Screen {
    pub fn all() -> &'static [Screen] {
        &[
            Screen::Dashboard,
            Screen::Sources,
            Screen::Repository,
            Screen::Snapshots,
            Screen::Restore,
            Screen::Scheduler,
            Screen::Logs,
            Screen::Settings,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Screen::Dashboard => "Dashboard",
            Screen::Sources => "Sources",
            Screen::Repository => "Repository",
            Screen::Snapshots => "Snapshots",
            Screen::Restore => "Restore",
            Screen::Scheduler => "Scheduler",
            Screen::Logs => "Logs",
            Screen::Settings => "Settings",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Screen::Dashboard => 0,
            Screen::Sources => 1,
            Screen::Repository => 2,
            Screen::Snapshots => 3,
            Screen::Restore => 4,
            Screen::Scheduler => 5,
            Screen::Logs => 6,
            Screen::Settings => 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Confirm { prompt: String, confirm_word: String, input: String, action: ConfirmAction },
    Input { prompt: String, input: String, action: InputAction },
    BackupRunning { progress: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    Prune,
    Restore,
    ForgetWithPrune,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    AddFlatPath,
    AddContainerPath,
    SetDockerVolumesPath,
    SetRepositoryUrl,
    SetBudgetTotal,
    SetBudgetWarning,
    SetBudgetCritical,
    EditScheduleCalendar,
    SetRestoreTargetPath,
    SetRestoreSourcePath,
}

pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub sources_config: SourcesConfig,
    pub schedules_config: SchedulesConfig,

    pub current_screen: Screen,
    pub mode: AppMode,
    pub should_quit: bool,

    pub sources_list_offset: usize,
    pub sources_selected_index: usize,
    pub sources_search: String,
    pub sources_search_active: bool,
    pub expanded_container: Option<PathBuf>,
    pub docker_path_missing: Option<PathBuf>,

    pub snapshots: Vec<Snapshot>,
    pub snapshots_selected_index: usize,
    pub snapshots_offset: usize,

    pub restore_target_input: String,
    pub restore_path_input: String,

    pub repo_stats: Option<ResticStats>,
    pub repo_reachable: Option<bool>,
    pub last_stats_check: Option<DateTime<Utc>>,

    pub scheduler_active: bool,

    pub log_entries: VecDeque<LogEntry>,
    pub log_offset: usize,

    pub settings_selected_index: usize,

    pub status_message: Option<(String, bool)>,
    pub last_backup_time: Option<DateTime<Utc>>,
    pub next_backup_time: Option<String>,

    pub background_tx: Option<mpsc::UnboundedSender<crate::tui::event::Event>>,
    pub backup_child_pid: Option<u32>,
    pub credentials: CredentialsConfig,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        sources_config: SourcesConfig,
        schedules_config: SchedulesConfig,
    ) -> Self {
        Self {
            config,
            sources_config,
            schedules_config,
            current_screen: Screen::Dashboard,
            mode: AppMode::Normal,
            should_quit: false,
            sources_list_offset: 0,
            sources_selected_index: 0,
            sources_search: String::new(),
            sources_search_active: false,
            expanded_container: None,
            docker_path_missing: None,
            snapshots: Vec::new(),
            snapshots_selected_index: 0,
            snapshots_offset: 0,
            restore_target_input: String::new(),
            restore_path_input: String::new(),
            repo_stats: None,
            repo_reachable: None,
            last_stats_check: None,
            scheduler_active: false,
            log_entries: VecDeque::new(),
            log_offset: 0,
            settings_selected_index: 0,
            status_message: None,
            last_backup_time: None,
            next_backup_time: None,
            background_tx: None,
            backup_child_pid: None,
            credentials: CredentialsConfig::load().unwrap_or_default(),
        }
    }

    pub fn navigate_to(&mut self, screen: Screen) {
        self.current_screen = screen;
        self.mode = AppMode::Normal;
    }

    pub fn next_screen(&mut self) {
        let screens = Screen::all();
        let idx = self.current_screen.index();
        self.current_screen = screens[(idx + 1) % screens.len()].clone();
    }

    pub fn prev_screen(&mut self) {
        let screens = Screen::all();
        let idx = self.current_screen.index();
        let prev = if idx == 0 { screens.len() - 1 } else { idx - 1 };
        self.current_screen = screens[prev].clone();
    }

    pub fn push_log(&mut self, level: LogLevel, message: String) {
        self.log_entries.push_back(LogEntry {
            timestamp: Utc::now(),
            level,
            message,
        });
        if self.log_entries.len() > MAX_LOG_LINES {
            self.log_entries.pop_front();
        }
    }

    pub fn refresh_scheduler_status(&self) {
        if let Some(tx) = self.background_tx.clone() {
            tokio::spawn(async move {
                let active = crate::scheduler::SystemdScheduler::is_active().await;
                let next_time = crate::scheduler::SystemdScheduler::next_trigger_time().await;
                let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                    crate::tui::event::BackgroundEvent::SchedulerStatus { active, next_time },
                ));
            });
        }
    }

    pub fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, is_error));
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn filtered_sources(&self) -> Vec<&BackupSource> {
        let query = self.sources_search.to_lowercase();

        let base_iter: Vec<&BackupSource> = match &self.expanded_container {
            Some(container_path) => {
                self.sources_config.sources.iter()
                    .filter(|s| {
                        match &s.kind {
                            SourceKind::Container { .. } => &s.path == container_path,
                            SourceKind::FlatPath { parent_container: Some(p) } => p == container_path,
                            SourceKind::FlatPath { parent_container: None } => false,
                        }
                    })
                    .collect()
            }
            None => {
                self.sources_config.sources.iter()
                    .filter(|s| {
                        match &s.kind {
                            SourceKind::Container { .. } => true,
                            SourceKind::FlatPath { parent_container: None } => true,
                            SourceKind::FlatPath { parent_container: Some(_) } => false,
                        }
                    })
                    .collect()
            }
        };

        if query.is_empty() {
            return base_iter;
        }
        base_iter.into_iter()
            .filter(|s| {
                s.label.to_lowercase().contains(&query)
                    || s.path.to_string_lossy().to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn handle_background_event(&mut self, event: BackgroundEvent) {
        match event {
            BackgroundEvent::DockerDiscoveryFound { container, children } => {
                self.docker_path_missing = None;
                self.sources_config.upsert(container);
                let mut added = 0usize;
                for child in children {
                    let is_new = !self.sources_config.sources.iter().any(|s| s.path == child.path);
                    self.sources_config.upsert_child(child);
                    if is_new { added += 1; }
                }
                if added > 0 {
                    self.push_log(LogLevel::Info, format!("Docker discovery: {} new volumes found (unapproved)", added));
                }
                let _ = self.sources_config.save();
            }
            BackgroundEvent::DockerPathMissing { searched } => {
                self.docker_path_missing = Some(searched.clone());
                self.push_log(
                    LogLevel::Warning,
                    format!(
                        "Docker volumes path not found: {}. This tool is primarily designed for Docker workloads. Press 'D' on the Sources screen to set a custom path.",
                        searched.display()
                    ),
                );
                self.set_status(
                    format!("Docker path not found: {} -- press 'D' to configure", searched.display()),
                    true,
                );
            }
            BackgroundEvent::ContainerChildrenScanned { container_path, children } => {
                let mut added = 0usize;
                for child in children {
                    let is_new = !self.sources_config.sources.iter().any(|s| s.path == child.path);
                    self.sources_config.upsert_child(child);
                    if is_new { added += 1; }
                }
                if added > 0 {
                    self.push_log(LogLevel::Info, format!("{} new children found in container", added));
                }
                let _ = self.sources_config.save();
            }
            BackgroundEvent::SnapshotsLoaded(snaps) => {
                let count = snaps.len();
                self.snapshots = snaps;
                self.push_log(LogLevel::Info, format!("Loaded {} snapshots", count));
            }
            BackgroundEvent::StatsLoaded(stats) => {
                self.last_stats_check = Some(Utc::now());
                let size_str = stats.display_size();
                self.repo_stats = Some(stats);
                self.repo_reachable = Some(true);
                self.push_log(LogLevel::Info, format!("Repository stats updated: {}", size_str));
            }
            BackgroundEvent::BackupProgress(progress) => {
                use crate::restic::ProgressEvent;
                match &progress {
                    ProgressEvent::BackupPid(pid) => {
                        self.backup_child_pid = Some(*pid);
                    }
                    ProgressEvent::BackupSummary(p) => {
                        self.mode = AppMode::Normal;
                        self.push_log(LogLevel::Info, format!("Backup complete: {}", p.display_progress()));
                        self.last_backup_time = Some(Utc::now());
                        let _ = self.sources_config.save();
                    }
                    ProgressEvent::BackupStatus(p) => {
                        self.mode = AppMode::BackupRunning {
                            progress: p.display_progress(),
                        };
                    }
                    ProgressEvent::Error(e) => {
                        self.push_log(LogLevel::Error, format!("Backup error: {}", e));
                    }
                    ProgressEvent::Finished => {
                        self.backup_child_pid = None;
                        self.mode = AppMode::Normal;
                    }
                    ProgressEvent::RawLine(line) => {
                        self.push_log(LogLevel::Debug, line.clone());
                    }
                }
            }
            BackgroundEvent::StatsFailed => {
                self.repo_reachable = Some(false);
            }
            BackgroundEvent::SchedulerStatus { active, next_time } => {
                self.scheduler_active = active;
                self.next_backup_time = next_time;
            }
            BackgroundEvent::PruneComplete(output) => {
                self.mode = AppMode::Normal;
                self.push_log(LogLevel::Info, format!("Prune complete: {}", output));
                self.set_status("Prune complete".into(), false);
            }
            BackgroundEvent::ForgetComplete { kept, removed } => {
                self.mode = AppMode::Normal;
                self.push_log(LogLevel::Info, format!("Forget complete: {} kept, {} removed", kept, removed));
                self.set_status(format!("Forget complete: {} removed", removed), false);
            }
            BackgroundEvent::RestoreComplete(output) => {
                self.mode = AppMode::Normal;
                self.push_log(LogLevel::Info, format!("Restore complete: {}", output));
                self.set_status("Restore complete".into(), false);
            }
            BackgroundEvent::Error(msg) => {
                self.push_log(LogLevel::Error, msg.clone());
                self.set_status(msg, true);
            }
            BackgroundEvent::OperationComplete(msg) => {
                self.push_log(LogLevel::Info, msg.clone());
                self.set_status(msg, false);
            }
        }
    }
}

pub struct App {
    pub state: AppState,
}

impl App {
    pub fn new(
        config: AppConfig,
        sources_config: SourcesConfig,
        schedules_config: SchedulesConfig,
    ) -> Self {
        Self {
            state: AppState::new(config, sources_config, schedules_config),
        }
    }
}
