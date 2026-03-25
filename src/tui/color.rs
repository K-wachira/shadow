use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Modifier;
use crate::tui;

pub fn hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color::Rgb(r, g, b)
}

pub fn bright() -> Style {
    Style::default().fg(Color::White)
}

pub fn bright_bold() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn muted() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn default() -> Style {
    Style::default().fg(tui::hex_color("#FFFFFF"))
}

pub fn very_dim() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM)
}