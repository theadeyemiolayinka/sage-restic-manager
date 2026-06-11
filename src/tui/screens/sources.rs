use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::config::sources::SourceState;
use crate::tui::app::AppState;
use crate::tui::theme::Theme;

pub fn render_sources(frame: &mut Frame, area: Rect, state: &AppState) {
    let has_banner = state.docker_path_missing.is_some();
    let is_inside_container = state.expanded_container.is_some();

    let constraints = if has_banner {
        vec![
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.clone())
        .split(area);

    let (banner_chunk, search_chunk, list_chunk, hints_chunk) = if has_banner {
        (Some(chunks[0]), chunks[1], chunks[2], chunks[3])
    } else {
        (None, chunks[0], chunks[1], chunks[2])
    };

    if let Some(bc) = banner_chunk {
        render_docker_missing_banner(frame, bc, state);
    }
    render_search_bar(frame, search_chunk, state, is_inside_container);
    render_source_list(frame, list_chunk, state, is_inside_container);
    render_source_hints(frame, hints_chunk, state, is_inside_container);
}

fn render_docker_missing_banner(frame: &mut Frame, area: Rect, state: &AppState) {
    let searched = state.docker_path_missing.as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let block = Block::default()
        .title(Span::styled(" Docker Path Not Found ", Theme::danger()))
        .borders(Borders::ALL)
        .border_style(Theme::danger());

    let text = vec![
        Line::from(vec![
            Span::styled(
                format!("  Docker volumes path '{}' was not found on this system.", searched),
                Theme::warning(),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  This tool is primarily designed for Docker workloads on Ubuntu servers.",
                Theme::dim(),
            ),
            Span::styled("  Press 'D' to set a custom path.", Theme::header()),
        ]),
    ];

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState, inside_container: bool) {
    let border_style = if state.sources_search_active {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let context_label = if inside_container {
        let container_label = state.expanded_container.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("container");
        format!(" Inside: {}  ", container_label)
    } else {
        String::new()
    };

    let display = if state.sources_search.is_empty() && !state.sources_search_active {
        Span::styled(format!("{}  Press '/' to search", context_label), Theme::dim())
    } else {
        Span::styled(
            format!("{}  Filter: {}_", context_label, state.sources_search),
            Theme::input_focused(),
        )
    };

    let title = if inside_container { " Search (Esc: back to top level) " } else { " Search " };
    let para = Paragraph::new(Line::from(display))
        .block(Block::default().title(title).borders(Borders::ALL).border_style(border_style));
    frame.render_widget(para, area);
}

fn render_source_list(frame: &mut Frame, area: Rect, state: &AppState, inside_container: bool) {
    let sources = state.filtered_sources();
    let total = sources.len();

    let items: Vec<ListItem> = sources.iter().map(|source| {
        let is_container = source.is_container();

        let state_indicator = if is_container {
            let child_count = state.sources_config.children_of(&source.path).len();
            let selected_children = state.sources_config.children_of(&source.path)
                .iter()
                .filter(|c| c.state == SourceState::Selected)
                .count();
            Span::styled(
                format!("[dir {}/{}] ", selected_children, child_count),
                Theme::header(),
            )
        } else {
            match source.state {
                SourceState::Selected => Span::styled("[+] ", Theme::state_selected()),
                SourceState::Unapproved => Span::styled("[?] ", Theme::state_unapproved()),
                SourceState::Ignored => Span::styled("[-] ", Theme::state_ignored()),
            }
        };

        let kind_col = Span::styled(
            format!("{:<10}", source.kind_label()),
            Theme::dim(),
        );

        let indent = if inside_container && !is_container {
            Span::raw("  ")
        } else {
            Span::raw("")
        };

        let label_style = if is_container {
            Theme::header()
        } else {
            Theme::normal()
        };

        let expand_hint = if is_container {
            Span::styled(" [Enter to browse]", Theme::dim())
        } else {
            Span::raw("")
        };

        let tag_str = if source.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", source.tags.join(","))
        };
        let label = Span::styled(
            format!("{:<30}", source.label),
            label_style,
        );
        let tags_span = Span::styled(
            format!("{:<14}", tag_str),
            Theme::dim(),
        );

        let size = Span::styled(
            format!("{:>10}", source.display_size()),
            Theme::dim(),
        );

        let last_backup = if is_container {
            Span::raw("")
        } else {
            match &source.last_backup {
                Some(t) => Span::styled(
                    format!("  {}", t.format("%Y-%m-%d")),
                    Theme::dim(),
                ),
                None => Span::styled("  never", Theme::dim()),
            }
        };

        ListItem::new(Line::from(vec![
            indent,
            state_indicator,
            kind_col,
            label,
            tags_span,
            expand_hint,
            size,
            last_backup,
        ]))
    }).collect();

    let title = if inside_container {
        let name = state.expanded_container.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("container");
        format!(" {} -- children ({}) ", name, total)
    } else {
        format!(" Backup Sources ({} entries) ", total)
    };

    let block = Block::default()
        .title(Span::styled(title, Theme::title()))
        .borders(Borders::ALL)
        .border_style(if inside_container { Theme::border_focused() } else { Theme::border() });

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected())
        .highlight_symbol(">");

    let mut list_state = ListState::default();
    if !sources.is_empty() {
        list_state.select(Some(
            state.sources_selected_index.min(sources.len().saturating_sub(1)),
        ));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_source_hints(frame: &mut Frame, area: Rect, state: &AppState, inside_container: bool) {
    let (hints_line1, hints_line2) = if inside_container {
        (
            Line::from(vec![
                Span::styled("  Enter", Theme::header()),
                Span::styled(": select/deselect  ", Theme::dim()),
                Span::styled("i", Theme::header()),
                Span::styled(": ignore  ", Theme::dim()),
                Span::styled("Esc", Theme::header()),
                Span::styled(": back to top level  ", Theme::dim()),
                Span::styled("s", Theme::header()),
                Span::styled(": save  ", Theme::dim()),
                Span::styled("r", Theme::header()),
                Span::styled(": rescan container", Theme::dim()),
            ]),
            Line::from(vec![
                Span::styled(
                    "  [?] unapproved -- must explicitly select each item. No bulk select.",
                    Theme::dim(),
                ),
            ]),
        )
    } else {
        (
            Line::from(vec![
                Span::styled("  Enter/Space", Theme::header()),
                Span::styled(": toggle  ", Theme::dim()),
                Span::styled("a", Theme::header()),
                Span::styled(": approve  ", Theme::dim()),
                Span::styled("i", Theme::header()),
                Span::styled(": ignore  ", Theme::dim()),
                Span::styled("d", Theme::header()),
                Span::styled(": discover  ", Theme::dim()),
                Span::styled("b", Theme::header()),
                Span::styled(": backup  ", Theme::dim()),
                Span::styled("+", Theme::header()),
                Span::styled(": add  ", Theme::dim()),
                Span::styled("s", Theme::header()),
                Span::styled(": save", Theme::dim()),
            ]),
            {
                let selected = state.sources_config.sources.iter()
                    .filter(|s| s.state == SourceState::Selected && !s.is_container())
                    .count();
                let unapproved = state.sources_config.sources.iter()
                    .filter(|s| s.state == SourceState::Unapproved && !s.is_container())
                    .count();
                let total_size = state.sources_config.total_selected_bytes();
                Line::from(vec![
                    Span::styled("  Flat ", Theme::dim()),
                    Span::styled("[flat]", Theme::header()),
                    Span::styled(" = entire directory as one backup. Container ", Theme::dim()),
                    Span::styled("[dir]", Theme::header()),
                    Span::styled(" = children managed individually.  ", Theme::dim()),
                    Span::styled("Sel: ", Theme::dim()),
                    Span::styled(format!("{}", selected), Theme::success()),
                    Span::styled("  Unapp: ", Theme::dim()),
                    Span::styled(
                        format!("{}", unapproved),
                        if unapproved > 0 { Theme::warning() } else { Theme::dim() },
                    ),
                    Span::styled("  Size: ", Theme::dim()),
                    Span::styled(bytesize::ByteSize(total_size).to_string_as(true), Theme::normal()),
                ])
            },
        )
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(area);

    frame.render_widget(Paragraph::new(hints_line1), chunks[0]);
    frame.render_widget(Paragraph::new(hints_line2), chunks[1]);
}
