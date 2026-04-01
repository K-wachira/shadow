use crate::tui::TuiAppState;
use crate::tui::bright;
use crate::tui::dim;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

pub fn render_input(f: &mut Frame, area: Rect, tui_state: &TuiAppState) {
    let prefix = if tui_state.memory_edit_mode {
        "edit>"
    } else {
        tui_state.assistant_state.input_prefix()
    };
    let cursor = Span::styled(
        "█",
        Style::default()
            .fg(Color::White)
            .bg(Color::Gray)
            .add_modifier(Modifier::SLOW_BLINK),
    );

    let line = if tui_state.input.is_empty() {
        Line::from(vec![
            Span::styled(format!("{} ", prefix), dim()),
            cursor,
            Span::raw(" "),
            Span::styled(
                if tui_state.memory_edit_mode {
                    "Type JSON value"
                } else {
                    "Type your message"
                },
                dim(),
            ),
        ])
    } else {
        let byte_idx = tui_state
            .input
            .char_indices()
            .nth(tui_state.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(tui_state.input.len());
        let before = tui_state.input[..byte_idx].to_string();
        let after = tui_state.input[byte_idx..].to_string();

        Line::from(vec![
            Span::styled(format!("{} ", prefix), dim()),
            Span::styled(before, bright()),
            cursor,
            Span::styled(after, bright()),
        ])
    };

    f.render_widget(Paragraph::new(line), area);
}
