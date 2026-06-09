use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;

pub fn render_scheduler(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(4),
        ])
        .split(area);

    render_schedule_status(frame, chunks[0], state);
    render_generated_units(frame, chunks[1], state);
    render_scheduler_hints(frame, chunks[2], state);
}

fn render_schedule_status(frame: &mut Frame, area: Rect, state: &AppState) {
    let active_schedule = state.schedules_config.active_schedule();

    let active_style = if state.scheduler_active {
        Theme::success()
    } else {
        Theme::warning()
    };

    let (freq_str, calendar_str) = match active_schedule {
        Some(s) => (s.frequency.to_string(), s.on_calendar_value()),
        None => ("(none configured)".into(), "(none)".into()),
    };

    let block = Block::default()
        .title(Span::styled(" Schedule Status ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  Systemd Timer:  ", Theme::dim()),
            Span::styled(
                if state.scheduler_active { "Active" } else { "Inactive" },
                active_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Frequency:      ", Theme::dim()),
            Span::styled(freq_str, Theme::normal()),
        ]),
        Line::from(vec![
            Span::styled("  OnCalendar:     ", Theme::dim()),
            Span::styled(calendar_str, Theme::normal()),
        ]),
        Line::from(vec![
            Span::styled("  Next Run:       ", Theme::dim()),
            Span::styled(
                state.next_backup_time.as_deref().unwrap_or("unknown"),
                Theme::normal(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Note: Schedules are managed via systemd timers. Root access required.",
                Theme::dim(),
            ),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_generated_units(frame: &mut Frame, area: Rect, state: &AppState) {
    let binary_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/usr/local/bin/sage-restic-manager".into());

    let active = state.schedules_config.active_schedule();
    let calendar = active.map(|s| s.on_calendar_value()).unwrap_or_else(|| "Mon,Thu 02:00:00".into());

    let block = Block::default()
        .title(Span::styled(" Generated Unit Preview ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let preview_text = format!(
        "sage-restic-manager.service: ExecStart={} backup --non-interactive\nsage-restic-manager.timer:   OnCalendar={}",
        binary_path,
        calendar
    );

    let para = Paragraph::new(preview_text).block(block);
    frame.render_widget(para, area);
}

fn render_scheduler_hints(frame: &mut Frame, area: Rect, _state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Actions ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  e", Theme::header()),
            Span::styled(": enable timer (requires root)  ", Theme::dim()),
            Span::styled("d", Theme::header()),
            Span::styled(": disable timer  ", Theme::dim()),
            Span::styled("i", Theme::header()),
            Span::styled(": install units  ", Theme::dim()),
            Span::styled("f", Theme::header()),
            Span::styled(": change frequency  ", Theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("  c", Theme::header()),
            Span::styled(": set custom OnCalendar  ", Theme::dim()),
            Span::styled("s", Theme::header()),
            Span::styled(": show systemctl status  ", Theme::dim()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
