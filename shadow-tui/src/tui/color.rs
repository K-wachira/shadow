use crate::tui;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;

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

pub fn selected_item_style() -> Style {
    Style::default()
        .fg(Color::Rgb(176, 185, 249))
        .bg(Color::Rgb(42, 44, 55))
}

pub fn default_item_style() -> Style {
    Style::default().fg(Color::Rgb(153, 153, 153))
}

pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn error_style() -> Style {
    Style::default().fg(Color::Red)
}

pub fn default() -> Style {
    Style::default().fg(tui::hex_color("#FFFFFF"))
}

pub fn very_dim() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_parses_white() {
        assert_eq!(hex_color("#FFFFFF"), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn hex_color_parses_black() {
        assert_eq!(hex_color("#000000"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn hex_color_parses_mixed() {
        assert_eq!(hex_color("#FF0080"), Color::Rgb(255, 0, 128));
    }

    #[test]
    fn hex_color_works_without_hash_prefix() {
        assert_eq!(hex_color("FF8000"), Color::Rgb(255, 128, 0));
    }

    #[test]
    fn hex_color_invalid_bytes_default_to_zero() {
        // Invalid hex chars fall back to 0 via unwrap_or(0)
        assert_eq!(hex_color("#GGGGGG"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn bright_returns_white_foreground() {
        assert_eq!(bright().fg, Some(Color::White));
    }

    #[test]
    fn bright_bold_has_bold_modifier() {
        assert!(bright_bold().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn muted_returns_gray_foreground() {
        assert_eq!(muted().fg, Some(Color::Gray));
    }

    #[test]
    fn dim_returns_dark_gray_foreground() {
        assert_eq!(dim().fg, Some(Color::DarkGray));
    }

    #[test]
    fn very_dim_has_dim_modifier() {
        assert!(very_dim().add_modifier.contains(Modifier::DIM));
    }
}
