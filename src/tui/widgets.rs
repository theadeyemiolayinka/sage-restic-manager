use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

use crate::tui::theme::Theme;

pub fn render_title_bar(frame: &mut Frame, area: Rect, title: &str, version: &str) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Theme::border())
        .style(Style::default().bg(Color::Rgb(10, 15, 30)));
    let text = Paragraph::new(Line::from(vec![
        Span::styled("  sage-restic-manager ", Theme::title()),
        Span::styled(format!("v{}  ", version), Theme::dim()),
        Span::styled(format!("[{}]", title), Theme::header()),
    ]))
    .block(block);
    frame.render_widget(text, area);
}

pub fn render_tab_bar(frame: &mut Frame, area: Rect, screens: &[&str], active: usize) {
    let spans: Vec<Span> = screens.iter().enumerate().map(|(i, name)| {
        if i == active {
            Span::styled(format!(" {} ", name), Theme::tab_active())
        } else {
            Span::styled(format!(" {} ", name), Theme::tab_inactive())
        }
    }).collect();
    let paragraph = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Theme::border()))
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

pub fn render_status_bar(frame: &mut Frame, area: Rect, status: Option<&(String, bool)>, hint: &str) {
    let (msg, style) = match status {
        Some((msg, true)) => (msg.clone(), Theme::danger()),
        Some((msg, false)) => (msg.clone(), Theme::success()),
        None => (hint.to_string(), Theme::dim()),
    };
    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled("  ", Theme::dim()),
        Span::styled(msg, style),
    ]))
    .block(Block::default().borders(Borders::TOP).border_style(Theme::border()));
    frame.render_widget(paragraph, area);
}

pub fn render_confirm_dialog(frame: &mut Frame, prompt: &str, confirm_word: &str, input: &str, is_match: bool) {
    let size = frame.area();
    let width = 60u16.min(size.width - 4);
    let height = 9u16;
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let border_style = if is_match { Theme::danger() } else { Theme::border_focused() };
    let block = Block::default()
        .title(Span::styled(" Confirmation Required ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner);

    let prompt_text = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(prompt, Theme::warning())),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(prompt_text, chunks[0]);

    let type_hint = Paragraph::new(Line::from(vec![
        Span::styled(format!("Type '{}' to confirm: ", confirm_word), Theme::normal()),
    ]));
    frame.render_widget(type_hint, chunks[1]);

    let input_style = if is_match { Theme::danger() } else { Theme::input_focused() };
    let input_widget = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {} ", input), input_style),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(input_style));
    frame.render_widget(input_widget, chunks[2]);
}

pub fn render_input_dialog(frame: &mut Frame, prompt: &str, input: &str) {
    let size = frame.area();
    let width = 60u16.min(size.width - 4);
    let height = 7u16;
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" Input ", Theme::title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(inner);

    let prompt_widget = Paragraph::new(Line::from(Span::styled(prompt, Theme::normal())));
    frame.render_widget(prompt_widget, chunks[0]);

    let input_widget = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {}_", input), Theme::input_focused()),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Theme::border_focused()));
    frame.render_widget(input_widget, chunks[1]);
}

pub fn render_loading_overlay(frame: &mut Frame, message: &str) {
    let size = frame.area();
    let width = 40u16.min(size.width - 4);
    let height = 3u16;
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());
    let paragraph = Paragraph::new(Line::from(Span::styled(message, Theme::normal())))
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

pub fn render_storage_gauge(frame: &mut Frame, area: Rect, used: u64, budget: u64, warning: u64, critical: u64) {
    let ratio = if budget == 0 { 0.0 } else { (used as f64 / budget as f64).min(1.0) };

    let style = if used >= critical {
        Theme::gauge_critical()
    } else if used >= warning {
        Theme::gauge_warning()
    } else {
        Theme::gauge_normal()
    };

    let label = format!(
        "{} / {} ({:.1}%)",
        bytesize::ByteSize(used).to_string_as(true),
        bytesize::ByteSize(budget).to_string_as(true),
        ratio * 100.0
    );

    let gauge = Gauge::default()
        .block(Block::default().title("Storage Budget").borders(Borders::ALL).border_style(Theme::border()))
        .gauge_style(style)
        .ratio(ratio)
        .label(label);

    frame.render_widget(gauge, area);
}

