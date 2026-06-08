use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, LogLevel};
use crate::tui::theme::Theme;

pub fn render_logs(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    render_log_list(frame, chunks[0], state);
    render_log_hints(frame, chunks[1], state);
}

fn render_log_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let entries: Vec<&crate::tui::app::LogEntry> = state.log_entries.iter().collect();
    let total = entries.len();

    let items: Vec<ListItem> = entries.iter().map(|entry| {
        let ts = entry.timestamp.format("%H:%M:%S").to_string();
        let level_style = match entry.level {
            LogLevel::Info => Theme::success(),
            LogLevel::Warning => Theme::warning(),
            LogLevel::Error => Theme::danger(),
            LogLevel::Debug => Theme::dim(),
        };

        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", ts), Theme::dim()),
            Span::styled(format!("{:<5} ", entry.level.to_string()), level_style),
            Span::styled(entry.message.clone(), Theme::normal()),
        ]))
    }).collect();

    let title = format!(" Logs ({} entries) ", total);
    let block = Block::default()
        .title(Span::styled(title, Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    if entries.is_empty() {
        let empty = Paragraph::new(Span::styled("  No log entries yet.", Theme::dim())).block(block);
        frame.render_widget(empty, area);
        return;
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    let mut list_state = ListState::default();
    let select_idx = if total > 0 {
        let offset = state.log_offset.min(total.saturating_sub(1));
        Some(total.saturating_sub(1).saturating_sub(offset))
    } else {
        None
    };
    list_state.select(select_idx);

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_log_hints(frame: &mut Frame, area: Rect, state: &AppState) {
    let hints = Paragraph::new(Line::from(vec![
        Span::styled("  g", Theme::header()),
        Span::styled(": scroll top  ", Theme::dim()),
        Span::styled("G", Theme::header()),
        Span::styled(": scroll bottom  ", Theme::dim()),
        Span::styled("Up/Down", Theme::header()),
        Span::styled(": scroll  ", Theme::dim()),
        Span::styled("e", Theme::header()),
        Span::styled(": export logs  ", Theme::dim()),
        Span::styled("c", Theme::header()),
        Span::styled(": clear  ", Theme::dim()),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Theme::border()));
    frame.render_widget(hints, area);
}
