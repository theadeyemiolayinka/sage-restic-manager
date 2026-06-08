use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::AppState;
use crate::tui::theme::Theme;

pub fn render_restore(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(4),
        ])
        .split(area);

    render_restore_form(frame, chunks[0], state);
    render_restore_preview(frame, chunks[1], state);
    render_restore_hints(frame, chunks[2], state);
}

fn render_restore_form(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected_snapshot = state.snapshots.get(state.snapshots_selected_index);

    let snapshot_info = match selected_snapshot {
        Some(snap) => format!(
            "{} - {} - {}",
            snap.short_id,
            snap.time.format("%Y-%m-%d %H:%M"),
            snap.display_paths()
        ),
        None => "(no snapshot selected - go to Snapshots screen first)".into(),
    };

    let target_display = if state.restore_target_input.is_empty() {
        "(enter target path)".into()
    } else {
        state.restore_target_input.clone()
    };

    let path_display = if state.restore_path_input.is_empty() {
        "(entire snapshot - leave blank for full restore)".into()
    } else {
        state.restore_path_input.clone()
    };

    let block = Block::default()
        .title(Span::styled(" Restore Configuration ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  Snapshot:     ", Theme::dim()),
            Span::styled(snapshot_info, Theme::normal()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Target Path:  ", Theme::dim()),
            Span::styled(target_display, Theme::input_focused()),
            Span::styled("  (press 't' to edit)", Theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("  Source Path:  ", Theme::dim()),
            Span::styled(path_display, Theme::input_focused()),
            Span::styled("  (press 'p' to edit, blank for full restore)", Theme::dim()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_restore_preview(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected_snapshot = state.snapshots.get(state.snapshots_selected_index);

    let block = Block::default()
        .title(Span::styled(" Restore Preview ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = match selected_snapshot {
        Some(snap) => {
            let target = if state.restore_target_input.is_empty() {
                Span::styled("(target not set)", Theme::danger())
            } else {
                Span::styled(state.restore_target_input.clone(), Theme::success())
            };
            vec![
                Line::from(vec![
                    Span::styled("  Will restore: ", Theme::dim()),
                    Span::styled(format!("snapshot {} ({})", snap.short_id, snap.age_description()), Theme::normal()),
                ]),
                Line::from(vec![
                    Span::styled("  Into:         ", Theme::dim()),
                    target,
                ]),
                Line::from(vec![
                    Span::styled("  Paths:        ", Theme::dim()),
                    Span::styled(
                        if state.restore_path_input.is_empty() {
                            "entire snapshot".into()
                        } else {
                            state.restore_path_input.clone()
                        },
                        Theme::normal(),
                    ),
                ]),
            ]
        }
        None => vec![
            Line::from(Span::styled("  Select a snapshot on the Snapshots screen first.", Theme::dim())),
        ],
    };

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_restore_hints(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(Span::styled(" Actions ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let lines = vec![
        Line::from(vec![
            Span::styled("  t", Theme::header()),
            Span::styled(": set target path  ", Theme::dim()),
            Span::styled("p", Theme::header()),
            Span::styled(": set source path  ", Theme::dim()),
            Span::styled("Enter", Theme::header()),
            Span::styled(": execute restore (type RESTORE to confirm)  ", Theme::danger()),
        ]),
        Line::from(vec![
            Span::styled("  WARNING: Restore will overwrite existing files in the target directory.", Theme::danger()),
        ]),
    ];

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
