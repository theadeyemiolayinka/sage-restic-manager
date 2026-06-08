use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use tokio::sync::mpsc;

use crate::tui::app::{App, AppMode, Screen};
use crate::tui::event::{BackgroundEvent, Event, EventHandler};
use crate::tui::input::handle_key;
use crate::tui::screens::{
    render_dashboard, render_logs, render_repository, render_restore,
    render_scheduler, render_settings, render_snapshots, render_sources,
};
use crate::tui::terminal::TerminalManager;
use crate::tui::widgets::{
    render_confirm_dialog, render_input_dialog, render_loading_overlay,
    render_status_bar, render_tab_bar, render_title_bar,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run(mut app: App) -> crate::error::Result<()> {
    let mut terminal = TerminalManager::new()?;
    let mut events = EventHandler::new(250);
    let event_tx = events.sender();
    app.state.background_tx = Some(event_tx.clone());

    trigger_initial_tasks(&app, &event_tx).await;

    loop {
        terminal.terminal.draw(|f| render(f, &app))?;

        if let Some(event) = events.next().await {
            match event {
                Event::Key(key) => {
                    handle_key(&mut app.state, key);
                }
                Event::Resize(_, _) => {}
                Event::Tick => {
                    app.state.clear_status();
                }
                Event::BackgroundTask(bg) => {
                    app.state.handle_background_event(bg);
                }
                _ => {}
            }
        }

        if app.state.should_quit {
            break;
        }
    }

    TerminalManager::restore()?;
    Ok(())
}

async fn trigger_initial_tasks(app: &App, tx: &mpsc::UnboundedSender<Event>) {
    let tx = tx.clone();
    let docker_path = app.state.sources_config.effective_docker_path();
    tokio::spawn(async move {
        use crate::discovery::{DockerDiscoveryResult, VolumeDiscovery};
        match VolumeDiscovery::discover_docker_volumes(&docker_path).await {
            DockerDiscoveryResult::Found { container, children } => {
                let _ = tx.send(Event::BackgroundTask(BackgroundEvent::DockerDiscoveryFound {
                    container,
                    children,
                }));
            }
            DockerDiscoveryResult::PathNotFound { searched } => {
                let _ = tx.send(Event::BackgroundTask(BackgroundEvent::DockerPathMissing {
                    searched,
                }));
            }
            DockerDiscoveryResult::PermissionDenied { path } => {
                let _ = tx.send(Event::BackgroundTask(BackgroundEvent::Error(
                    format!("Permission denied reading Docker volumes path: {}", path.display()),
                )));
            }
        }
    });
}

fn render(frame: &mut Frame, app: &App) {
    let state = &app.state;

    let size = frame.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(size);

    render_title_bar(frame, main_chunks[0], state.current_screen.label(), VERSION);

    let screen_labels: Vec<&str> = Screen::all().iter().map(|s| s.label()).collect();
    render_tab_bar(frame, main_chunks[1], &screen_labels, state.current_screen.index());

    let content_area = main_chunks[2];

    match state.current_screen {
        Screen::Dashboard => render_dashboard(frame, content_area, state),
        Screen::Sources => render_sources(frame, content_area, state),
        Screen::Repository => render_repository(frame, content_area, state),
        Screen::Snapshots => render_snapshots(frame, content_area, state),
        Screen::Restore => render_restore(frame, content_area, state),
        Screen::Scheduler => render_scheduler(frame, content_area, state),
        Screen::Logs => render_logs(frame, content_area, state),
        Screen::Settings => render_settings(frame, content_area, state),
    }

    let hint = screen_hint(state.current_screen.clone());
    render_status_bar(frame, main_chunks[3], state.status_message.as_ref(), hint);

    match &state.mode {
        AppMode::Confirm { prompt, confirm_word, input, .. } => {
            let is_match = input == confirm_word;
            render_confirm_dialog(frame, prompt, confirm_word, input, is_match);
        }
        AppMode::Input { prompt, input, .. } => {
            render_input_dialog(frame, prompt, input);
        }
        AppMode::Loading { message } => {
            render_loading_overlay(frame, message);
        }
        AppMode::BackupRunning { progress } => {
            render_loading_overlay(frame, &format!("Backup running: {}", progress));
        }
        AppMode::Normal => {}
    }
}

fn screen_hint(screen: Screen) -> &'static str {
    match screen {
        Screen::Dashboard => "Tab: next screen  1-8: jump  q: quit",
        Screen::Sources => "Enter: toggle  i: ignore  a: approve  /: search  +: add path  s: save",
        Screen::Repository => "i: init  c: check  b: backup now  p: prune  r: refresh stats",
        Screen::Snapshots => "Up/Down: navigate  r: refresh  R: go to restore",
        Screen::Restore => "t: set target  p: set path  Enter: execute restore",
        Screen::Scheduler => "e: enable  d: disable  i: install  f: frequency  c: custom calendar",
        Screen::Logs => "Up/Down: scroll  g: top  G: bottom  c: clear  e: export",
        Screen::Settings => "Up/Down: navigate  Enter: edit value",
    }
}
