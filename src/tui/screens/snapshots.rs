use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;

pub fn render_snapshots(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    render_snapshot_list(frame, chunks[0], state);
    render_snapshot_hints(frame, chunks[1], state);
}

fn render_snapshot_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let snapshots = &state.snapshots;

    let items: Vec<ListItem> = snapshots.iter().map(|snap| {
        let time_str = snap.time.format("%Y-%m-%d %H:%M").to_string();
        let short_id = Span::styled(format!("{:<12}", snap.short_id), Theme::header());
        let time = Span::styled(format!("{:<18}", time_str), Theme::normal());
        let host = Span::styled(format!("{:<20}", snap.hostname), Theme::dim());
        let paths = Span::styled(
            format!("{:.40}", snap.display_paths()),
            Theme::dim(),
        );
        let age = Span::styled(format!("  {}", snap.age_description()), Theme::dim());

        ListItem::new(Line::from(vec![
            Span::raw("  "),
            short_id,
            time,
            host,
            paths,
            age,
        ]))
    }).collect();

    let title = format!(" Snapshots ({}) ", snapshots.len());
    let block = Block::default()
        .title(Span::styled(title, Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    if snapshots.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No snapshots found. Run a backup first.", Theme::dim())),
            Line::from(""),
            Line::from(Span::styled("  Press 'r' to refresh.", Theme::dim())),
        ])
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<12}", "ID"), Theme::header()),
            Span::styled(format!("{:<18}", "Time"), Theme::header()),
            Span::styled(format!("{:<20}", "Host"), Theme::header()),
            Span::styled("Paths", Theme::header()),
        ]))
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT).border_style(Theme::border())),
        inner_chunks[0],
    );

    let list = List::new(items)
        .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT).border_style(Theme::border()))
        .highlight_style(Theme::selected())
        .highlight_symbol(">");

    let mut list_state = ListState::default();
    if !state.snapshots.is_empty() {
        list_state.select(Some(
            state.snapshots_selected_index.min(state.snapshots.len().saturating_sub(1)),
        ));
    }

    frame.render_stateful_widget(list, inner_chunks[1], &mut list_state);
}

fn render_snapshot_hints(frame: &mut Frame, area: Rect, _state: &AppState) {
    let hints = Paragraph::new(Line::from(vec![
        Span::styled("  r", Theme::header()),
        Span::styled(": refresh  ", Theme::dim()),
        Span::styled("Enter", Theme::header()),
        Span::styled(": view detail  ", Theme::dim()),
        Span::styled("R", Theme::header()),
        Span::styled(": restore snapshot  ", Theme::dim()),
        Span::styled("f", Theme::header()),
        Span::styled(": forget (dry-run)  ", Theme::dim()),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Theme::border()));
    frame.render_widget(hints, area);
}
