use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;

pub fn render_repository(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Min(4),
        ])
        .split(area);

    render_repo_config(frame, chunks[0], state);
    render_retention_policy(frame, chunks[1], state);
    render_repo_actions(frame, chunks[2], state);
}

fn render_repo_config(frame: &mut Frame, area: Rect, state: &AppState) {
    let cfg = &state.config.repository;

    let backend_str = cfg.backend.to_string();
    let url_display = if cfg.url.is_empty() { "(not set)".into() } else { cfg.url.clone() };
    let bucket_display = if cfg.bucket.is_empty() { "(not set)".into() } else { cfg.bucket.clone() };
    let creds = &state.credentials;
    let key_display = if creds.access_key_id.is_empty() { "(not set)".into() } else { format!("{}...", &creds.access_key_id[..creds.access_key_id.len().min(8)]) };
    let password_display: String = if creds.repository_password.is_empty() { "(not set)".into() } else { "****".into() };

    let endpoint_display = cfg.endpoint.as_deref().unwrap_or("(default)").to_string();

    let block = Block::default()
        .title(Span::styled(" Repository Configuration ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![Span::styled("  Backend:       ", Theme::dim()), Span::styled(backend_str, Theme::normal())]),
        Line::from(vec![Span::styled("  Endpoint URL:  ", Theme::dim()), Span::styled(url_display, Theme::normal())]),
        Line::from(vec![Span::styled("  Bucket:        ", Theme::dim()), Span::styled(bucket_display, Theme::normal())]),
        Line::from(vec![Span::styled("  Region:        ", Theme::dim()), Span::styled(cfg.region.clone(), Theme::normal())]),
        Line::from(vec![Span::styled("  Custom EP:     ", Theme::dim()), Span::styled(endpoint_display, Theme::normal())]),
        Line::from(vec![Span::styled("  Path:          ", Theme::dim()), Span::styled(cfg.path.clone(), Theme::normal())]),
        Line::from(vec![Span::styled("  Access Key:    ", Theme::dim()), Span::styled(key_display, Theme::dim())]),
        Line::from(vec![Span::styled("  Password:      ", Theme::dim()), Span::styled(password_display, Theme::dim())]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_retention_policy(frame: &mut Frame, area: Rect, state: &AppState) {
    let ret = &state.config.retention;

    let block = Block::default()
        .title(Span::styled(" Retention Policy ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let fmt = |v: &Option<u32>| v.map(|n| n.to_string()).unwrap_or_else(|| "-".into());

    let lines = vec![
        Line::from(vec![
            Span::styled("  Keep Last: ", Theme::dim()), Span::styled(fmt(&ret.keep_last), Theme::normal()),
            Span::styled("   Daily: ", Theme::dim()), Span::styled(fmt(&ret.keep_daily), Theme::normal()),
            Span::styled("   Weekly: ", Theme::dim()), Span::styled(fmt(&ret.keep_weekly), Theme::normal()),
            Span::styled("   Monthly: ", Theme::dim()), Span::styled(fmt(&ret.keep_monthly), Theme::normal()),
            Span::styled("   Yearly: ", Theme::dim()), Span::styled(fmt(&ret.keep_yearly), Theme::normal()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_repo_actions(frame: &mut Frame, area: Rect, _state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Actions ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  i", Theme::header()),
            Span::styled(": init repository  ", Theme::dim()),
            Span::styled("c", Theme::header()),
            Span::styled(": check repository  ", Theme::dim()),
            Span::styled("f", Theme::header()),
            Span::styled(": forget+prune  ", Theme::dim()),
            Span::styled("p", Theme::header()),
            Span::styled(": prune (type PRUNE)", Theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("  b", Theme::header()),
            Span::styled(": backup now  ", Theme::dim()),
            Span::styled("r", Theme::header()),
            Span::styled(": refresh stats  ", Theme::dim()),
            Span::styled("e", Theme::header()),
            Span::styled(": edit config  ", Theme::dim()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
