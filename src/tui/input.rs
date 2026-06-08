use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::path::PathBuf;
use crate::config::sources::SourceState;
use crate::tui::app::{AppMode, AppState, ConfirmAction, InputAction, LogLevel, Screen};
use crate::tui::event::BackgroundEvent;

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> bool {
    match state.mode.clone() {
        AppMode::Normal => handle_normal(state, key),
        AppMode::Confirm { prompt, confirm_word, mut input, action } => {
            handle_confirm(state, key, prompt, confirm_word, input, action)
        }
        AppMode::Input { prompt, mut input, action } => {
            handle_input(state, key, prompt, input, action)
        }
        AppMode::Loading { .. } => false,
        AppMode::BackupRunning { .. } => {
            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                state.push_log(LogLevel::Warning, "Backup interrupt requested".into());
            }
            false
        }
    }
}

fn handle_normal(state: &mut AppState, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => {
                state.should_quit = true;
                return true;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('q') if state.current_screen == Screen::Dashboard => {
            state.should_quit = true;
            return true;
        }
        KeyCode::Tab => state.next_screen(),
        KeyCode::BackTab => state.prev_screen(),
        KeyCode::Char('1') => state.navigate_to(Screen::Dashboard),
        KeyCode::Char('2') => state.navigate_to(Screen::Sources),
        KeyCode::Char('3') => state.navigate_to(Screen::Repository),
        KeyCode::Char('4') => state.navigate_to(Screen::Snapshots),
        KeyCode::Char('5') => state.navigate_to(Screen::Restore),
        KeyCode::Char('6') => state.navigate_to(Screen::Scheduler),
        KeyCode::Char('7') => state.navigate_to(Screen::Logs),
        KeyCode::Char('8') => state.navigate_to(Screen::Settings),
        _ => handle_screen_key(state, key),
    }
    false
}

fn handle_screen_key(state: &mut AppState, key: KeyEvent) {
    match state.current_screen {
        Screen::Sources => handle_sources_key(state, key),
        Screen::Snapshots => handle_snapshots_key(state, key),
        Screen::Restore => handle_restore_key(state, key),
        Screen::Repository => handle_repository_key(state, key),
        Screen::Scheduler => handle_scheduler_key(state, key),
        Screen::Logs => handle_logs_key(state, key),
        Screen::Settings => handle_settings_key(state, key),
        Screen::Dashboard => handle_dashboard_key(state, key),
    }
}

fn handle_sources_key(state: &mut AppState, key: KeyEvent) {
    if state.sources_search_active {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                state.sources_search_active = false;
            }
            KeyCode::Backspace => {
                state.sources_search.pop();
            }
            KeyCode::Char(c) => {
                state.sources_search.push(c);
                state.sources_selected_index = 0;
            }
            _ => {}
        }
        return;
    }

    let filtered_len = state.filtered_sources().len();
    let inside_container = state.expanded_container.is_some();

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.sources_selected_index > 0 {
                state.sources_selected_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if filtered_len > 0 && state.sources_selected_index < filtered_len - 1 {
                state.sources_selected_index += 1;
            }
        }
        KeyCode::PageUp => {
            state.sources_selected_index = state.sources_selected_index.saturating_sub(10);
        }
        KeyCode::PageDown => {
            if filtered_len > 0 {
                state.sources_selected_index = (state.sources_selected_index + 10).min(filtered_len - 1);
            }
        }
        KeyCode::Char('/') => {
            state.sources_search_active = true;
            state.sources_search.clear();
        }
        KeyCode::Esc => {
            if inside_container {
                state.expanded_container = None;
                state.sources_selected_index = 0;
                state.sources_search.clear();
            } else {
                state.sources_search.clear();
                state.sources_selected_index = 0;
            }
        }
        KeyCode::Enter => {
            sources_enter_action(state);
        }
        KeyCode::Char(' ') => {
            toggle_source_at_cursor(state);
        }
        KeyCode::Char('i') => {
            ignore_source(state);
        }
        KeyCode::Char('s') => {
            match state.sources_config.save() {
                Ok(_) => state.set_status("Sources saved".into(), false),
                Err(e) => state.set_status(format!("Save failed: {}", e), true),
            }
        }
        KeyCode::Char('r') if inside_container => {
            rescan_current_container(state);
        }
        KeyCode::Char('d') if !inside_container => {
            run_docker_discovery(state);
        }
        KeyCode::Char('D') if !inside_container => {
            state.mode = AppMode::Input {
                prompt: "Enter Docker volumes path (e.g. /var/lib/docker/volumes):".into(),
                input: state.sources_config.effective_docker_path().to_string_lossy().to_string(),
                action: InputAction::SetDockerVolumesPath,
            };
        }
        KeyCode::Char('+') if !inside_container => {
            state.mode = AppMode::Input {
                prompt: "Add path as: [f]lat (back up this path entirely) or [c]ontainer (browse children): Type 'f' or 'c':".into(),
                input: String::new(),
                action: InputAction::AddFlatPath,
            };
        }
        _ => {}
    }
}

fn sources_enter_action(state: &mut AppState) {
    let idx = state.sources_selected_index;
    let filtered_paths: Vec<PathBuf> = state.filtered_sources()
        .iter()
        .map(|s| s.path.clone())
        .collect();

    let path = match filtered_paths.get(idx) {
        Some(p) => p.clone(),
        None => return,
    };

    let is_container = state.sources_config.sources.iter()
        .find(|s| s.path == path)
        .map(|s| s.is_container())
        .unwrap_or(false);

    if is_container {
        state.expanded_container = Some(path.clone());
        state.sources_selected_index = 0;
        state.sources_search.clear();
        state.set_status(
            format!("Browsing children of: {}  (Esc to return)", path.display()),
            false,
        );
    } else {
        toggle_source_at_cursor(state);
    }
}

fn rescan_current_container(state: &mut AppState) {
    if let Some(container_path) = state.expanded_container.clone() {
        if let Some(tx) = state.background_tx.clone() {
            let path = container_path.clone();
            tokio::spawn(async move {
                use crate::discovery::{ContainerScanResult, VolumeDiscovery};
                match VolumeDiscovery::scan_container_children(&path).await {
                    ContainerScanResult::Ok { container_path, children } => {
                        let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                            BackgroundEvent::ContainerChildrenScanned { container_path, children },
                        ));
                    }
                    ContainerScanResult::PermissionDenied => {
                        let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                            BackgroundEvent::Error(format!("Permission denied reading: {}", path.display())),
                        ));
                    }
                    ContainerScanResult::PathNotFound => {
                        let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                            BackgroundEvent::Error(format!("Path no longer exists: {}", path.display())),
                        ));
                    }
                }
            });
            state.set_status("Rescanning container...".into(), false);
        }
    }
}

fn run_docker_discovery(state: &mut AppState) {
    if let Some(tx) = state.background_tx.clone() {
        let docker_path = state.sources_config.effective_docker_path();
        tokio::spawn(async move {
            use crate::discovery::{DockerDiscoveryResult, VolumeDiscovery};
            match VolumeDiscovery::discover_docker_volumes(&docker_path).await {
                DockerDiscoveryResult::Found { container, children } => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::DockerDiscoveryFound { container, children },
                    ));
                }
                DockerDiscoveryResult::PathNotFound { searched } => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::DockerPathMissing { searched },
                    ));
                }
                DockerDiscoveryResult::PermissionDenied { path } => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::Error(format!("Permission denied: {}", path.display())),
                    ));
                }
            }
        });
        state.set_status("Running Docker volume discovery...".into(), false);
    }
}

fn toggle_source_at_cursor(state: &mut AppState) {
    let idx = state.sources_selected_index;
    let filtered_paths: Vec<PathBuf> = state.filtered_sources()
        .iter()
        .map(|s| s.path.clone())
        .collect();

    if let Some(path) = filtered_paths.get(idx) {
        let path = path.clone();
        let is_container = state.sources_config.sources.iter()
            .find(|s| s.path == path)
            .map(|s| s.is_container())
            .unwrap_or(false);
        if is_container {
            state.set_status("Containers cannot be selected directly. Browse children with Enter.".into(), true);
            return;
        }
        let log_msg = if let Some(source) = state.sources_config.find_by_path_mut(&path) {
            match source.state {
                SourceState::Selected => {
                    source.state = SourceState::Unapproved;
                    Some((LogLevel::Info, format!("Deselected: {}", source.label)))
                }
                SourceState::Unapproved | SourceState::Ignored => {
                    source.state = SourceState::Selected;
                    Some((LogLevel::Info, format!("Selected: {}", source.label)))
                }
            }
        } else {
            None
        };
        if let Some((level, msg)) = log_msg {
            state.push_log(level, msg);
        }
    }
}

fn ignore_source(state: &mut AppState) {
    let idx = state.sources_selected_index;
    let filtered_paths: Vec<PathBuf> = state.filtered_sources()
        .iter()
        .map(|s| s.path.clone())
        .collect();

    if let Some(path) = filtered_paths.get(idx) {
        let path = path.clone();
        let log_msg = if let Some(source) = state.sources_config.find_by_path_mut(&path) {
            if !source.is_container() {
                source.state = SourceState::Ignored;
                Some(format!("Ignored: {}", source.label))
            } else {
                None
            }
        } else {
            None
        };
        if let Some(msg) = log_msg {
            state.push_log(LogLevel::Info, msg);
        }
    }
}

fn handle_snapshots_key(state: &mut AppState, key: KeyEvent) {
    let len = state.snapshots.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.snapshots_selected_index > 0 {
                state.snapshots_selected_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if len > 0 && state.snapshots_selected_index < len - 1 {
                state.snapshots_selected_index += 1;
            }
        }
        KeyCode::Char('r') => {
            state.set_status("Refreshing snapshots...".into(), false);
        }
        KeyCode::Char('R') => {
            state.navigate_to(Screen::Restore);
        }
        _ => {}
    }
}

fn handle_restore_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('t') => {
            state.mode = AppMode::Input {
                prompt: "Enter restore target path:".into(),
                input: state.restore_target_input.clone(),
                action: InputAction::AddFlatPath,
            };
        }
        KeyCode::Enter => {
            if state.restore_target_input.is_empty() {
                state.set_status("Set a target path first (press 't')".into(), true);
                return;
            }
            if state.snapshots.is_empty() {
                state.set_status("No snapshot selected".into(), true);
                return;
            }
            state.mode = AppMode::Confirm {
                prompt: format!(
                    "Restore snapshot '{}' to '{}'? This will overwrite existing files.",
                    state.snapshots.get(state.snapshots_selected_index)
                        .map(|s| s.short_id.clone())
                        .unwrap_or_default(),
                    state.restore_target_input,
                ),
                confirm_word: "RESTORE".into(),
                input: String::new(),
                action: ConfirmAction::Restore,
            };
        }
        _ => {}
    }
}

fn handle_repository_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('p') => {
            state.mode = AppMode::Confirm {
                prompt: "This will permanently prune unreferenced data from the repository.".into(),
                confirm_word: "PRUNE".into(),
                input: String::new(),
                action: ConfirmAction::Prune,
            };
        }
        KeyCode::Char('r') => {
            state.set_status("Refreshing repository stats...".into(), false);
        }
        _ => {}
    }
}

fn handle_scheduler_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('c') => {
            state.mode = AppMode::Input {
                prompt: "Enter systemd OnCalendar expression (e.g. Mon,Thu 02:00:00):".into(),
                input: state.schedules_config.active_schedule()
                    .map(|s| s.on_calendar_value())
                    .unwrap_or_default(),
                action: InputAction::EditScheduleCalendar,
            };
        }
        _ => {}
    }
}

fn handle_logs_key(state: &mut AppState, key: KeyEvent) {
    let len = state.log_entries.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.log_offset = state.log_offset.saturating_add(1).min(len.saturating_sub(1));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.log_offset = state.log_offset.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            state.log_offset = len.saturating_sub(1);
        }
        KeyCode::Char('G') => {
            state.log_offset = 0;
        }
        KeyCode::Char('c') => {
            state.log_entries.clear();
            state.log_offset = 0;
            state.set_status("Logs cleared".into(), false);
        }
        _ => {}
    }
}

fn handle_settings_key(state: &mut AppState, key: KeyEvent) {
    let items_len = 11;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_selected_index > 0 {
                state.settings_selected_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.settings_selected_index < items_len - 1 {
                state.settings_selected_index += 1;
            }
        }
        KeyCode::Enter => {
            let (prompt, action) = settings_entry_action(state.settings_selected_index);
            if let Some(action) = action {
                state.mode = AppMode::Input {
                    prompt,
                    input: String::new(),
                    action,
                };
            }
        }
        _ => {}
    }
}

fn settings_entry_action(idx: usize) -> (String, Option<InputAction>) {
    match idx {
        0 => ("Enter repository URL:".into(), Some(InputAction::SetRepositoryUrl)),
        5 => ("Enter storage budget in GB (e.g. 8.0):".into(), Some(InputAction::SetBudgetTotal)),
        6 => ("Enter warning threshold in GB (e.g. 6.0):".into(), Some(InputAction::SetBudgetWarning)),
        7 => ("Enter critical threshold in GB (e.g. 7.5):".into(), Some(InputAction::SetBudgetCritical)),
        _ => ("Not editable from TUI - edit config.toml directly.".into(), None),
    }
}

fn handle_dashboard_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('r') => {
            state.set_status("Refreshing...".into(), false);
        }
        _ => {}
    }
}

fn handle_confirm(
    state: &mut AppState,
    key: KeyEvent,
    prompt: String,
    confirm_word: String,
    mut input: String,
    action: ConfirmAction,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.mode = AppMode::Normal;
            state.set_status("Cancelled".into(), false);
        }
        KeyCode::Backspace => {
            input.pop();
            state.mode = AppMode::Confirm { prompt, confirm_word, input, action };
        }
        KeyCode::Char(c) => {
            input.push(c);
            if input == confirm_word {
                let confirmed_action = action.clone();
                state.mode = AppMode::Normal;
                execute_confirm_action(state, confirmed_action);
            } else {
                state.mode = AppMode::Confirm { prompt, confirm_word, input, action };
            }
        }
        _ => {}
    }
    false
}

fn execute_confirm_action(state: &mut AppState, action: ConfirmAction) {
    match action {
        ConfirmAction::Prune => {
            state.push_log(LogLevel::Info, "Prune operation initiated".into());
            state.set_status("Prune initiated (see logs)".into(), false);
        }
        ConfirmAction::Restore => {
            state.push_log(LogLevel::Info, "Restore operation initiated".into());
            state.set_status("Restore initiated".into(), false);
        }
        ConfirmAction::ForgetWithPrune => {
            state.push_log(LogLevel::Info, "Forget+prune initiated".into());
        }
        ConfirmAction::DeleteRepository => {
            state.push_log(LogLevel::Warning, "Repository deletion initiated".into());
        }
        ConfirmAction::SelectMany => {
            state.push_log(LogLevel::Info, "Bulk selection confirmed".into());
        }
    }
}

fn handle_input(
    state: &mut AppState,
    key: KeyEvent,
    prompt: String,
    mut input: String,
    action: InputAction,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            input.pop();
            state.mode = AppMode::Input { prompt, input, action };
        }
        KeyCode::Enter => {
            let value = input.trim().to_string();
            execute_input_action(state, action, value);
            state.mode = AppMode::Normal;
        }
        KeyCode::Char(c) => {
            input.push(c);
            state.mode = AppMode::Input { prompt, input, action };
        }
        _ => {}
    }
    false
}

fn execute_input_action(state: &mut AppState, action: InputAction, value: String) {
    match action {
        InputAction::AddFlatPath => {
            let trimmed = value.trim().to_lowercase();
            if trimmed == "f" {
                state.mode = AppMode::Input {
                    prompt: "Enter path to add as flat (entire directory backed up as one unit):".into(),
                    input: String::new(),
                    action: InputAction::AddFlatPath,
                };
                return;
            } else if trimmed == "c" {
                state.mode = AppMode::Input {
                    prompt: "Enter path to add as container (children individually managed):".into(),
                    input: String::new(),
                    action: InputAction::AddContainerPath,
                };
                return;
            }
            if value.is_empty() { return; }
            let path = PathBuf::from(&value);
            if !path.exists() {
                state.set_status(format!("Path does not exist: {}", value), true);
                return;
            }
            let label = path.file_name().and_then(|n| n.to_str()).unwrap_or(&value).to_string();
            let source = crate::config::BackupSource::new_flat_standalone(path, label);
            state.sources_config.upsert(source);
            state.push_log(LogLevel::Info, format!("Added flat path: {}", value));
            let _ = state.sources_config.save();
            state.set_status(format!("Added as flat path: {} (unapproved)", value), false);
        }
        InputAction::AddContainerPath => {
            if value.is_empty() { return; }
            let path = PathBuf::from(&value);
            if !path.exists() {
                state.set_status(format!("Path does not exist: {}", value), true);
                return;
            }
            let label = path.file_name().and_then(|n| n.to_str()).unwrap_or(&value).to_string();
            let source = crate::config::BackupSource::new_container(
                path.clone(),
                label,
                crate::config::ContainerOrigin::CustomDirectory,
            );
            state.sources_config.upsert(source);
            state.push_log(LogLevel::Info, format!("Added container: {}", value));
            let _ = state.sources_config.save();
            if let Some(tx) = state.background_tx.clone() {
                tokio::spawn(async move {
                    use crate::discovery::{ContainerScanResult, VolumeDiscovery};
                    match VolumeDiscovery::scan_container_children(&path).await {
                        ContainerScanResult::Ok { container_path, children } => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::ContainerChildrenScanned { container_path, children },
                            ));
                        }
                        _ => {}
                    }
                });
            }
            state.set_status(format!("Added container: {} -- children scanned (unapproved)", value), false);
        }
        InputAction::SetDockerVolumesPath => {
            if value.is_empty() { return; }
            let path = PathBuf::from(&value);
            state.sources_config.docker_volumes_path = Some(path.clone());
            let _ = state.sources_config.save();
            state.push_log(LogLevel::Info, format!("Docker volumes path set to: {}", value));
            if !path.exists() {
                state.set_status(
                    format!("Path saved but not found: {}. Check the path and run discovery (d).", value),
                    true,
                );
            } else {
                state.docker_path_missing = None;
                state.set_status(format!("Docker path set: {}. Press 'd' to discover volumes.", value), false);
            }
        }
        InputAction::SetRepositoryUrl => {
            state.config.repository.url = value.clone();
            let _ = state.config.save();
            state.set_status(format!("Repository URL set to: {}", value), false);
        }
        InputAction::SetBudgetTotal => {
            if let Ok(gb) = value.parse::<f64>() {
                state.config.budget.total_bytes = (gb * 1024.0 * 1024.0 * 1024.0) as u64;
                let _ = state.config.save();
                state.set_status(format!("Budget set to {:.1} GB", gb), false);
            } else {
                state.set_status("Invalid number".into(), true);
            }
        }
        InputAction::SetBudgetWarning => {
            if let Ok(gb) = value.parse::<f64>() {
                state.config.budget.warning_bytes = (gb * 1024.0 * 1024.0 * 1024.0) as u64;
                let _ = state.config.save();
                state.set_status(format!("Warning threshold set to {:.1} GB", gb), false);
            } else {
                state.set_status("Invalid number".into(), true);
            }
        }
        InputAction::SetBudgetCritical => {
            if let Ok(gb) = value.parse::<f64>() {
                state.config.budget.critical_bytes = (gb * 1024.0 * 1024.0 * 1024.0) as u64;
                let _ = state.config.save();
                state.set_status(format!("Critical threshold set to {:.1} GB", gb), false);
            } else {
                state.set_status("Invalid number".into(), true);
            }
        }
        InputAction::EditScheduleCalendar => {
            if value.is_empty() { return; }
            if let Some(sched) = state.schedules_config.schedules.first_mut() {
                sched.on_calendar = Some(value.clone());
                sched.frequency = crate::config::ScheduleFrequency::Custom;
            } else {
                let mut sched = crate::config::ScheduleConfig::default();
                sched.on_calendar = Some(value.clone());
                sched.frequency = crate::config::ScheduleFrequency::Custom;
                state.schedules_config.schedules.push(sched);
            }
            let _ = state.schedules_config.save();
            state.set_status(format!("OnCalendar set to: {}", value), false);
        }
        _ => {}
    }
}
