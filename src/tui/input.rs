use chrono::Utc;
#[cfg(unix)]
use libc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::path::PathBuf;
use crate::config::sources::SourceState;
use crate::tui::app::{AppMode, AppState, ConfirmAction, InputAction, LogLevel, Screen};
use crate::tui::event::BackgroundEvent;

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> bool {
    match state.mode.clone() {
        AppMode::Normal => handle_normal(state, key),
        AppMode::Confirm { prompt, confirm_word, input, action } => {
            handle_confirm(state, key, prompt, confirm_word, input, action)
        }
        AppMode::Input { prompt, input, action } => {
            handle_input(state, key, prompt, input, action)
        }
        AppMode::BackupRunning { .. } => {
            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Some(pid) = state.backup_child_pid {
                    #[cfg(unix)]
                    unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM); }
                    state.push_log(LogLevel::Warning, format!("Sent SIGTERM to backup process (pid {})", pid));
                    state.set_status("Backup interrupt sent".into(), true);
                } else {
                    state.push_log(LogLevel::Warning, "Backup interrupt requested but pid unknown".into());
                }
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
        KeyCode::Char('a') => {
            approve_source(state);
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
        KeyCode::Char('b') if !inside_container => {
            trigger_backup(state);
        }
        KeyCode::Char('t') => {
            let filtered: Vec<_> = state.filtered_sources().iter().map(|s| s.path.clone()).collect();
            if let Some(path) = filtered.get(state.sources_selected_index) {
                let current_tags = state.sources_config.find_by_path_mut(path)
                    .map(|s| s.tags.join(", "))
                    .unwrap_or_default();
                state.mode = AppMode::Input {
                    prompt: "Enter tags (comma-separated):".into(),
                    input: current_tags,
                    action: InputAction::SetSourceTags,
                };
            }
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
                    ContainerScanResult::Ok { container_path: _, children } => {
                        let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                            BackgroundEvent::ContainerChildrenScanned { children },
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

fn approve_source(state: &mut AppState) {
    let idx = state.sources_selected_index;
    let filtered_paths: Vec<PathBuf> = state.filtered_sources()
        .iter()
        .map(|s| s.path.clone())
        .collect();

    if let Some(path) = filtered_paths.get(idx) {
        let path = path.clone();
        let log_msg = if let Some(source) = state.sources_config.find_by_path_mut(&path) {
            if source.state == SourceState::Unapproved {
                source.state = SourceState::Selected;
                Some(format!("Approved: {}", source.label))
            } else {
                None
            }
        } else {
            None
        };
        if let Some(msg) = log_msg {
            state.push_log(LogLevel::Info, msg);
        } else {
            state.set_status("Source is already approved or is a container".into(), false);
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
        KeyCode::Enter => {
            if let Some(snap) = state.snapshots.get(state.snapshots_selected_index) {
                state.set_status(
                    format!("Snapshot {} | {} | {} | {}",
                        snap.short_id,
                        snap.time.format("%Y-%m-%d %H:%M"),
                        snap.hostname,
                        snap.display_paths()),
                    false,
                );
            }
        }
        KeyCode::Char('f') => {
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.set_status("Running forget dry-run...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    match client.forget(&config.retention, true).await {
                        Ok(results) => {
                            let kept: usize = results.iter().map(|r| r.keep.len()).sum();
                            let removed: usize = results.iter().map(|r| r.remove.len()).sum();
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete(
                                    format!("Forget dry-run: would keep {}, remove {}", kept, removed)
                                ),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Forget dry-run failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('r') => {
            refresh_snapshots(state);
        }
        KeyCode::Char('R') => {
            state.navigate_to(Screen::Restore);
        }
        _ => {}
    }
}

fn refresh_snapshots(state: &mut AppState) {
    if let Some(tx) = state.background_tx.clone() {
        let config = state.config.clone();
        let creds = state.credentials.clone();
        tokio::spawn(async move {
            let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
            match client.snapshots().await {
                Ok(snaps) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::SnapshotsLoaded(snaps),
                    ));
                }
                Err(e) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::Error(format!("Snapshot refresh failed: {}", e)),
                    ));
                }
            }
        });
        state.set_status("Refreshing snapshots...".into(), false);
    }
}

fn refresh_stats(state: &mut AppState) {
    if let Some(tx) = state.background_tx.clone() {
        let config = state.config.clone();
        let creds = state.credentials.clone();
        tokio::spawn(async move {
            let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
            match client.stats().await {
                Ok(stats) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::StatsLoaded(stats),
                    ));
                }
                Err(e) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::StatsFailed,
                    ));
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::Error(format!("Stats refresh failed: {}", e)),
                    ));
                }
            }
        });
        state.set_status("Refreshing repository stats...".into(), false);
    }
}

fn handle_restore_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('t') => {
            state.mode = AppMode::Input {
                prompt: "Enter restore target path:".into(),
                input: state.restore_target_input.clone(),
                action: InputAction::SetRestoreTargetPath,
            };
        }
        KeyCode::Char('p') => {
            state.mode = AppMode::Input {
                prompt: "Enter source path to restore (leave empty to restore full snapshot):".into(),
                input: state.restore_path_input.clone(),
                action: InputAction::SetRestoreSourcePath,
            };
        }
        KeyCode::Enter => {
            if state.restore_target_input.is_empty() {
                state.set_status("Set a target path first (press 't')".into(), true);
                return;
            }
            let snap = match state.snapshots.get(state.snapshots_selected_index) {
                Some(s) => s.short_id.clone(),
                None => {
                    state.set_status("No snapshot selected".into(), true);
                    return;
                }
            };
            state.mode = AppMode::Confirm {
                prompt: format!(
                    "Restore snapshot '{}' to '{}'? This will overwrite existing files.",
                    snap,
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
        KeyCode::Char('i') => {
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.set_status("Initialising repository...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    match client.init().await {
                        Ok(out) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete(format!("Repository initialised: {}", out.lines().next().unwrap_or("ok"))),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Init failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('c') => {
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.set_status("Checking repository...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    match client.check(false).await {
                        Ok(result) => {
                            let msg = if result.ok {
                                "Repository check passed".into()
                            } else {
                                format!("Repository check FAILED: {}", result.output.lines().next().unwrap_or(""))
                            };
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete(msg),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Check failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('b') => {
            state.navigate_to(Screen::Sources);
            state.set_status("Select sources then press Enter to start backup".into(), false);
        }
        KeyCode::Char('f') => {
            state.mode = AppMode::Confirm {
                prompt: "This will forget snapshots per retention policy, then prune. Type FORGET to confirm:".into(),
                confirm_word: "FORGET".into(),
                input: String::new(),
                action: ConfirmAction::ForgetWithPrune,
            };
        }
        KeyCode::Char('p') => {
            state.mode = AppMode::Confirm {
                prompt: "This will permanently prune unreferenced data from the repository.".into(),
                confirm_word: "PRUNE".into(),
                input: String::new(),
                action: ConfirmAction::Prune,
            };
        }
        KeyCode::Char('e') => {
            state.navigate_to(Screen::Settings);
            state.set_status("Edit configuration in Settings".into(), false);
        }
        KeyCode::Char('r') => {
            refresh_stats(state);
        }
        _ => {}
    }
}

fn handle_scheduler_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('e') => {
            if let Some(tx) = state.background_tx.clone() {
                state.set_status("Enabling timer...".into(), false);
                tokio::spawn(async move {
                    match crate::scheduler::SystemdScheduler::enable().await {
                        Ok(()) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete("Timer enabled".into()),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Enable failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('d') => {
            if let Some(tx) = state.background_tx.clone() {
                state.set_status("Disabling timer...".into(), false);
                tokio::spawn(async move {
                    match crate::scheduler::SystemdScheduler::disable().await {
                        Ok(()) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete("Timer disabled".into()),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Disable failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('i') => {
            if let Some(tx) = state.background_tx.clone() {
                let binary = std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "sage-restic-manager".into());
                let sched = state.schedules_config.active_schedule().cloned()
                    .unwrap_or_default();
                state.set_status("Installing systemd units...".into(), false);
                tokio::spawn(async move {
                    match crate::scheduler::SystemdScheduler::install(&sched, &binary).await {
                        Ok(()) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete("Systemd units installed".into()),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Install failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('f') => {
            state.mode = AppMode::Input {
                prompt: "Enter schedule frequency: daily / weekly / twiceweekly / custom:".into(),
                input: String::new(),
                action: InputAction::EditScheduleCalendar,
            };
        }
        KeyCode::Char('s') => {
            if let Some(tx) = state.background_tx.clone() {
                state.set_status("Fetching systemctl status...".into(), false);
                tokio::spawn(async move {
                    match crate::scheduler::SystemdScheduler::status().await {
                        Ok(output) => {
                            let summary = output.lines().next().unwrap_or("(no output)").to_string();
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::OperationComplete(format!("Status: {}", summary)),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Status failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
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
            state.log_offset = state.log_offset.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.log_offset = state.log_offset.saturating_add(1).min(len.saturating_sub(1));
        }
        KeyCode::Char('g') => {
            state.log_offset = 0;
        }
        KeyCode::Char('G') => {
            state.log_offset = len.saturating_sub(1);
        }
        KeyCode::Char('c') => {
            state.log_entries.clear();
            state.log_offset = 0;
            state.set_status("Logs cleared".into(), false);
        }
        KeyCode::Char('e') => {
            let log_dir = match crate::config::log_dir() {
                Ok(d) => d,
                Err(e) => {
                    state.set_status(format!("Cannot resolve log dir: {}", e), true);
                    return;
                }
            };
            let filename = format!("sage-logs-{}.txt", Utc::now().format("%Y%m%dT%H%M%SZ"));
            let export_path = log_dir.join(&filename);
            let lines: Vec<String> = state.log_entries.iter()
                .map(|e| format!("[{}] [{:?}] {}", e.timestamp.format("%Y-%m-%dT%H:%M:%SZ"), e.level, e.message))
                .collect();
            match std::fs::write(&export_path, lines.join("\n")) {
                Ok(()) => state.set_status(format!("Logs exported to {}", export_path.display()), false),
                Err(e) => state.set_status(format!("Export failed: {}", e), true),
            }
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
        KeyCode::Char('b') => {
            trigger_backup(state);
        }
        KeyCode::Char('r') => {
            refresh_stats(state);
            refresh_snapshots(state);
        }
        _ => {}
    }
}

fn trigger_backup(state: &mut AppState) {
    let selected = state.sources_config.selected_paths();
    if selected.is_empty() {
        state.set_status("No sources selected. Go to Sources and approve/select paths first.".into(), true);
        return;
    }
    for source in state.sources_config.sources.iter_mut() {
        if source.state == crate::config::sources::SourceState::Selected {
            source.last_backup_status = Some(crate::config::sources::BackupStatus::Running);
        }
    }
    if let Some(tx) = state.background_tx.clone() {
        let config = state.config.clone();
        let creds = state.credentials.clone();
        let sources = state.sources_config.clone();
        state.mode = AppMode::BackupRunning { progress: "Starting...".into() };
        state.set_status(format!("Backup started ({} sources)", selected.len()), false);
        tokio::spawn(async move {
            let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
            if !client.is_available().await {
                let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                    BackgroundEvent::Error("restic binary not found".into()),
                ));
                return;
            }
            let mut tags = vec!["sage-restic-manager".to_string()];
            for source in sources.selected_sources() {
                for tag in &source.tags {
                    if !tags.contains(tag) {
                        tags.push(tag.clone());
                    }
                }
            }
            let exclude_patterns: Vec<String> = sources.selected_sources()
                .iter()
                .flat_map(|s| s.exclude_patterns.clone())
                .collect();
            let (prog_tx, mut prog_rx) = tokio::sync::mpsc::channel(128);
            let tx2 = tx.clone();
            tokio::spawn(async move {
                while let Some(ev) = prog_rx.recv().await {
                    let _ = tx2.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::BackupProgress(ev),
                    ));
                }
            });
            match client.backup_with_progress(&selected, &tags, &exclude_patterns, prog_tx).await {
                Ok(_) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::OperationComplete("Backup complete".into()),
                    ));
                }
                Err(e) => {
                    let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                        BackgroundEvent::Error(format!("Backup failed: {}", e)),
                    ));
                }
            }
        });
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
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.push_log(LogLevel::Info, "Prune started".into());
                state.set_status("Pruning...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    match client.prune().await {
                        Ok(out) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::PruneComplete(out),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Prune failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        ConfirmAction::Restore => {
            let snap = match state.snapshots.get(state.snapshots_selected_index) {
                Some(s) => s.clone(),
                None => {
                    state.set_status("No snapshot selected".into(), true);
                    return;
                }
            };
            let target = state.restore_target_input.clone();
            let source = if state.restore_path_input.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(&state.restore_path_input))
            };
            if target.is_empty() {
                state.set_status("No restore target set".into(), true);
                return;
            }
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.push_log(LogLevel::Info, format!("Restoring snapshot {} to {}", snap.short_id, target));
                state.set_status("Restoring...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    let restore_target = crate::restic::RestoreTarget {
                        snapshot_id: snap.id.clone(),
                        source_path: source,
                        target_path: std::path::PathBuf::from(&target),
                    };
                    match client.restore(&restore_target).await {
                        Ok(out) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::RestoreComplete(out),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Restore failed: {}", e)),
                            ));
                        }
                    }
                });
            }
        }
        ConfirmAction::ForgetWithPrune => {
            if let Some(tx) = state.background_tx.clone() {
                let config = state.config.clone();
                let creds = state.credentials.clone();
                state.push_log(LogLevel::Info, "Forget+prune started".into());
                state.set_status("Running forget + prune...".into(), false);
                tokio::spawn(async move {
                    let client = crate::restic::ResticClient::new_with_creds(&config, &creds);
                    match client.forget(&config.retention, false).await {
                        Ok(results) => {
                            let kept: usize = results.iter().map(|r| r.keep.len()).sum();
                            let removed: usize = results.iter().map(|r| r.remove.len()).sum();
                            if removed > 0 {
                                match client.prune().await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                            BackgroundEvent::Error(format!("Prune after forget failed: {}", e)),
                                        ));
                                        return;
                                    }
                                }
                            }
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::ForgetComplete { kept, removed },
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::Error(format!("Forget failed: {}", e)),
                            ));
                        }
                    }
                });
            }
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
                        ContainerScanResult::Ok { container_path: _, children } => {
                            let _ = tx.send(crate::tui::event::Event::BackgroundTask(
                                BackgroundEvent::ContainerChildrenScanned { children },
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
            if let Err(e) = crate::config::schedules::validate_on_calendar(&value) {
                state.set_status(format!("Invalid schedule expression: {}", e), true);
                return;
            }
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
        InputAction::SetRestoreTargetPath => {
            state.restore_target_input = value.clone();
            state.set_status(format!("Restore target set to: {}", value), false);
        }
        InputAction::SetRestoreSourcePath => {
            state.restore_path_input = value.clone();
            if value.is_empty() {
                state.set_status("Source path cleared (full snapshot will be restored)".into(), false);
            } else {
                state.set_status(format!("Restore source path set to: {}", value), false);
            }
        }
        InputAction::SetSourceTags => {
            let filtered: Vec<_> = state.filtered_sources().iter().map(|s| s.path.clone()).collect();
            if let Some(path) = filtered.get(state.sources_selected_index) {
                let tag_list: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let label = if let Some(source) = state.sources_config.find_by_path_mut(path) {
                    source.tags = tag_list.clone();
                    Some(source.label.clone())
                } else {
                    None
                };
                if let Some(label) = label {
                    let _ = state.sources_config.save();
                    state.set_status(format!("Tags for {}: {}", label, tag_list.join(", ")), false);
                }
            }
        }
    }
}
