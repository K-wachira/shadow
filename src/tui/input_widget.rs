use crate::tui::dim;
use crate::tui::bright;

use crate::tui::AppState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_input(f: &mut Frame, area: Rect, state: &AppState) {
    let prefix = state.assistant_state.input_prefix();
    let cursor = Span::styled(
        "█",
        Style::default()
            .fg(Color::White)
            .bg(Color::Gray)
            .add_modifier(Modifier::SLOW_BLINK),
    );

    let line = if state.input.is_empty() {
        Line::from(vec![
            Span::styled(format!("{} ", prefix), dim()),
            cursor,
            Span::raw(" "),
            Span::styled("Type your message", dim()),
        ])
    } else {
        let byte_idx = state
            .input
            .char_indices()
            .nth(state.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(state.input.len());
        let before = state.input[..byte_idx].to_string();
        let after = state.input[byte_idx..].to_string();

        Line::from(vec![
            Span::styled(format!("{} ", prefix), dim()),
            Span::styled(before, bright()),
            cursor,
            Span::styled(after, bright()),
        ])
    };

    f.render_widget(Paragraph::new(line), area);
}
