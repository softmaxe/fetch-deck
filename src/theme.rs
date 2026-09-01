use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub const BACKGROUND: Color = Color::Rgb(0x1e, 0x1e, 0x2e);
pub const SURFACE: Color = Color::Rgb(0x18, 0x18, 0x25);
pub const PANEL: Color = Color::Rgb(0x11, 0x11, 0x1b);
pub const FOREGROUND: Color = Color::Rgb(0xcd, 0xd6, 0xf4);
pub const MUTED: Color = Color::Rgb(0xa6, 0xad, 0xc8);
pub const FAINT: Color = Color::Rgb(0x7f, 0x84, 0x9c);
pub const BORDER: Color = Color::Rgb(0x45, 0x47, 0x5a);
pub const FOCUS: Color = Color::Rgb(0xcb, 0xa6, 0xf7);
pub const HEADING: Color = Color::Rgb(0xb4, 0xbe, 0xfe);
pub const KEY: Color = Color::Rgb(0x89, 0xb4, 0xfa);
pub const PROGRESS: Color = Color::Rgb(0x89, 0xb4, 0xfa);
pub const SUCCESS: Color = Color::Rgb(0xa6, 0xe3, 0xa1);
pub const WORKING: Color = Color::Rgb(0xf9, 0xe2, 0xaf);
pub const ERROR: Color = Color::Rgb(0xf3, 0x8b, 0xa8);
pub const SELECTION_BACKGROUND: Color = Color::Rgb(0x31, 0x32, 0x44);
pub const HOVER_BACKGROUND: Color = Color::Rgb(0x45, 0x47, 0x5a);

pub fn panel(title: &str, focused: bool) -> Block<'_> {
    let border = if focused { FOCUS } else { BORDER };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(SURFACE).fg(FOREGROUND))
}

pub fn selected() -> Style {
    Style::default()
        .fg(FOREGROUND)
        .bg(SELECTION_BACKGROUND)
        .add_modifier(Modifier::BOLD)
}

pub fn hovered() -> Style {
    Style::default()
        .fg(HEADING)
        .bg(HOVER_BACKGROUND)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_hovered() -> Style {
    Style::default()
        .fg(FOCUS)
        .bg(HOVER_BACKGROUND)
        .add_modifier(Modifier::BOLD)
}

pub fn muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn faint() -> Style {
    Style::default().fg(FAINT)
}
