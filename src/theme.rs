use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub const BACKGROUND: Color = Color::Rgb(20, 24, 29);
pub const PANEL: Color = Color::Rgb(32, 39, 46);
pub const TEXT: Color = Color::Rgb(231, 226, 215);
pub const MUTED: Color = Color::Rgb(139, 151, 161);
pub const BROADCAST_RED: Color = Color::Rgb(224, 96, 83);
pub const BUFFER_GOLD: Color = Color::Rgb(210, 165, 74);
pub const SIGNAL_BLUE: Color = Color::Rgb(104, 159, 204);
pub const READY_GREEN: Color = Color::Rgb(108, 179, 156);

pub fn panel(title: &str, focused: bool) -> Block<'_> {
    let border = if focused { BROADCAST_RED } else { PANEL };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(BACKGROUND).fg(TEXT))
}

pub fn selected() -> Style {
    Style::default()
        .fg(BACKGROUND)
        .bg(BUFFER_GOLD)
        .add_modifier(Modifier::BOLD)
}

pub fn dimmed() -> Style {
    Style::default().fg(MUTED).bg(BACKGROUND)
}
