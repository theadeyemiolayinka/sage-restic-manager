use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;
use crate::tui::widgets::render_storage_gauge;

pub fn render_dashboard(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Min(4),
        ])
        .split(area);

    render_overview(frame, chunks[0], state);
    render_budget_gauge(frame, chunks[1], state);
    render_trend(frame, chunks[2], state);
}

fn render_overview(frame: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    render_repo_status(frame, cols[0], state);
    render_backup_status(frame, cols[1], state);
}

fn render_repo_status(frame: &mut Frame, area: Rect, state: &AppState) {
    let reachable_line = match state.repo_reachable {
        Some(true) => Span::styled("Reachable", Theme::success()),
        Some(false) => Span::styled("Unreachable", Theme::danger()),
        None => Span::styled("Unknown", Theme::dim()),
    };

    let (size_line, snap_line) = match &state.repo_stats {
        Some(stats) => (
            Span::styled(stats.display_size(), Theme::normal()),
            Span::styled(
                stats.snapshots_count.map(|n| n.to_string()).unwrap_or_else(|| "-".into()),
                Theme::normal(),
            ),
        ),
        None => (
            Span::styled("-", Theme::dim()),
            Span::styled("-", Theme::dim()),
        ),
    };

    let last_check = state.last_stats_check
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "never".into());

    let block = Block::default()
        .title(Span::styled(" Repository ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![Span::styled("  Status:     ", Theme::dim()), reachable_line]),
        Line::from(vec![Span::styled("  Size:       ", Theme::dim()), size_line]),
        Line::from(vec![Span::styled("  Snapshots:  ", Theme::dim()), snap_line]),
        Line::from(vec![
            Span::styled("  Checked:    ", Theme::dim()),
            Span::styled(last_check, Theme::dim()),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_backup_status(frame: &mut Frame, area: Rect, state: &AppState) {
    let last_backup = state.last_backup_time
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "never".into());

    let next_backup = state.next_backup_time
        .as_deref()
        .unwrap_or("not scheduled");

    let selected_count = state.sources_config.sources.iter()
        .filter(|s| s.state == crate::config::SourceState::Selected)
        .count();

    let unapproved_count = state.sources_config.sources.iter()
        .filter(|s| s.state == crate::config::SourceState::Unapproved)
        .count();

    let total_selected_size = state.sources_config.total_selected_bytes();
    let size_str = bytesize::ByteSize(total_selected_size).to_string_as(true);

    let unapproved_style = if unapproved_count > 0 {
        Theme::warning()
    } else {
        Theme::dim()
    };

    let block = Block::default()
        .title(Span::styled(" Backup Status ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  Last Backup:     ", Theme::dim()),
            Span::styled(last_backup, Theme::normal()),
        ]),
        Line::from(vec![
            Span::styled("  Next Backup:     ", Theme::dim()),
            Span::styled(next_backup.to_string(), Theme::normal()),
        ]),
        Line::from(vec![
            Span::styled("  Selected:        ", Theme::dim()),
            Span::styled(format!("{} sources ({})", selected_count, size_str), Theme::success()),
        ]),
        Line::from(vec![
            Span::styled("  Unapproved:      ", Theme::dim()),
            Span::styled(format!("{}", unapproved_count), unapproved_style),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_budget_gauge(frame: &mut Frame, area: Rect, state: &AppState) {
    let budget = &state.config.budget;
    let used = state.repo_stats.as_ref().map(|s| s.total_size).unwrap_or(0);

    if !budget.enabled {
        let block = Block::default()
            .title(Span::styled(" Storage Budget ", Theme::title()))
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let para = Paragraph::new(Span::styled("  Budget tracking disabled", Theme::dim())).block(block);
        frame.render_widget(para, area);
        return;
    }

    render_storage_gauge(
        frame,
        area,
        used,
        budget.total_bytes,
        budget.warning_bytes,
        budget.critical_bytes,
    );
}

fn render_trend(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Storage Trend ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let budget = &state.config.budget;
    let used = state.repo_stats.as_ref().map(|s| s.total_size).unwrap_or(0);
    let history = &state.storage_history;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Budget:    ", Theme::dim()),
            Span::styled(
                format!("{:.1} GB", budget.budget_gib()),
                Theme::normal(),
            ),
            Span::styled("   Warning:  ", Theme::dim()),
            Span::styled(
                format!("{:.1} GB", budget.warning_gib()),
                Theme::warning(),
            ),
            Span::styled("   Critical: ", Theme::dim()),
            Span::styled(
                format!("{:.1} GB", budget.critical_gib()),
                Theme::danger(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Used:      ", Theme::dim()),
            Span::styled(
                bytesize::ByteSize(used).to_string_as(true),
                if used >= budget.critical_bytes {
                    Theme::critical()
                } else if used >= budget.warning_bytes {
                    Theme::warning()
                } else {
                    Theme::success()
                },
            ),
        ]),
    ];

    if let Some(rate) = history.growth_rate_bytes_per_day() {
        let rate_str = bytesize::ByteSize(rate as u64).to_string_as(true);
        lines.push(Line::from(vec![
            Span::styled("  Growth:    ", Theme::dim()),
            Span::styled(format!("{} / day", rate_str), Theme::normal()),
        ]));
    }

    if budget.enabled {
        if let Some(days) = history.days_until_budget(budget.total_bytes) {
            let days_str = if days < 1.0 {
                "budget exceeded".into()
            } else {
                format!("{:.0} days until budget", days)
            };
            let style = if days < 7.0 { Theme::danger() } else if days < 30.0 { Theme::warning() } else { Theme::success() };
            lines.push(Line::from(vec![
                Span::styled("  Forecast:  ", Theme::dim()),
                Span::styled(days_str, style),
            ]));
        }
    }

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
