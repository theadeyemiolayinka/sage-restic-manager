use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn border() -> Style {
        Style::default().fg(Color::Rgb(70, 130, 180))
    }

    pub fn border_focused() -> Style {
        Style::default().fg(Color::Rgb(100, 200, 255))
    }

    pub fn title() -> Style {
        Style::default().fg(Color::Rgb(200, 230, 255)).add_modifier(Modifier::BOLD)
    }

    pub fn header() -> Style {
        Style::default().fg(Color::Rgb(150, 200, 255)).add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::default().fg(Color::Black).bg(Color::Rgb(70, 130, 180)).add_modifier(Modifier::BOLD)
    }

    pub fn normal() -> Style {
        Style::default().fg(Color::Rgb(200, 200, 200))
    }

    pub fn dim() -> Style {
        Style::default().fg(Color::Rgb(120, 120, 120))
    }

    pub fn success() -> Style {
        Style::default().fg(Color::Rgb(80, 200, 120))
    }

    pub fn warning() -> Style {
        Style::default().fg(Color::Rgb(255, 200, 50))
    }

    pub fn danger() -> Style {
        Style::default().fg(Color::Rgb(220, 80, 80))
    }

    pub fn critical() -> Style {
        Style::default().fg(Color::Rgb(255, 50, 50)).add_modifier(Modifier::BOLD)
    }

    pub fn gauge_normal() -> Style {
        Style::default().fg(Color::Rgb(70, 130, 180))
    }

    pub fn gauge_warning() -> Style {
        Style::default().fg(Color::Rgb(255, 200, 50))
    }

    pub fn gauge_critical() -> Style {
        Style::default().fg(Color::Rgb(220, 80, 80))
    }

    pub fn tab_active() -> Style {
        Style::default().fg(Color::Rgb(100, 200, 255)).add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::default().fg(Color::Rgb(100, 100, 130))
    }

    pub fn input_focused() -> Style {
        Style::default().fg(Color::White).bg(Color::Rgb(30, 40, 60))
    }

    pub fn state_selected() -> Style {
        Style::default().fg(Color::Rgb(80, 200, 120)).add_modifier(Modifier::BOLD)
    }

    pub fn state_unapproved() -> Style {
        Style::default().fg(Color::Rgb(255, 200, 50))
    }

    pub fn state_ignored() -> Style {
        Style::default().fg(Color::Rgb(120, 120, 120))
    }

}
