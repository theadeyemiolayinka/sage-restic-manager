use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;

const SETTINGS_ITEMS: &[&str] = &[
    "Repository URL",
    "Bucket",
    "Region",
    "Access Key ID",
    "Repository Password",
    "Storage Budget Total (GB)",
    "Storage Budget Warning (GB)",
    "Storage Budget Critical (GB)",
    "Restic Binary Path",
    "Update Channel",
    "Log Level",
];

pub fn render_settings(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(area);

    render_settings_list(frame, chunks[0], state);
    render_settings_detail(frame, chunks[1], state);
}

fn render_settings_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = SETTINGS_ITEMS.iter().enumerate().map(|(_i, name)| {
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(*name, Theme::normal()),
        ]))
    }).collect();

    let block = Block::default()
        .title(Span::styled(" Settings ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected())
        .highlight_symbol(">");

    let mut list_state = ListState::default();
    list_state.select(Some(state.settings_selected_index.min(SETTINGS_ITEMS.len().saturating_sub(1))));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_settings_detail(frame: &mut Frame, area: Rect, state: &AppState) {
    let cfg = &state.config;
    let idx = state.settings_selected_index.min(SETTINGS_ITEMS.len().saturating_sub(1));

    let creds = &state.credentials;
    let (label, current_value, description) = match idx {
        0 => ("Repository URL", cfg.repository.url.clone(), "S3-compatible endpoint URL (e.g. https://account.r2.cloudflarestorage.com)"),
        1 => ("Bucket", cfg.repository.bucket.clone(), "Bucket or repository name"),
        2 => ("Region", cfg.repository.region.clone(), "Region (use 'auto' for Cloudflare R2)"),
        3 => ("Access Key ID", {
            if creds.access_key_id.is_empty() { "(not set)".into() }
            else { format!("{}...", &creds.access_key_id[..creds.access_key_id.len().min(8)]) }
        }, "S3 access key ID"),
        4 => ("Repository Password", if creds.repository_password.is_empty() { "(not set)".into() } else { "****".into() }, "Restic repository encryption password"),
        5 => ("Budget Total", format!("{:.1} GB", cfg.budget.budget_gib()), "Maximum storage budget in gigabytes"),
        6 => ("Budget Warning", format!("{:.1} GB", cfg.budget.warning_gib()), "Warning threshold in gigabytes"),
        7 => ("Budget Critical", format!("{:.1} GB", cfg.budget.critical_gib()), "Critical threshold in gigabytes"),
        8 => ("Restic Binary", cfg.restic_binary.clone(), "Path to restic binary (default: restic)"),
        9 => ("Update Channel", cfg.update_channel.to_string(), "stable or beta"),
        10 => ("Log Level", cfg.log_level.clone(), "Tracing log level: error, warn, info, debug, trace"),
        _ => ("", String::new(), ""),
    };

    let block = Block::default()
        .title(Span::styled(" Setting Detail ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {}", label), Theme::header())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Current value:  ", Theme::dim()),
            Span::styled(current_value, Theme::normal()),
        ]),
        Line::from(""),
        Line::from(Span::styled(format!("  {}", description), Theme::dim())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Theme::dim()),
            Span::styled("Enter", Theme::header()),
            Span::styled(" to edit this value.", Theme::dim()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
