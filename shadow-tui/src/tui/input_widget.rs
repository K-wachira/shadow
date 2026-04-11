use crate::tui::SLASH_COMMANDS;
use crate::tui::TuiAppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use shadow_utils::color;

const CURSOR_SENTINEL: char = '\u{0}';

pub fn render_input(f: &mut Frame, area: Rect, tui_state: &TuiAppState) {
    let lines = wrapped_input_lines(area.width, tui_state);
    let visible_start = lines.len().saturating_sub(area.height as usize);
    let visible_lines = lines.into_iter().skip(visible_start).collect::<Vec<_>>();
    f.render_widget(Paragraph::new(visible_lines), area);
}

pub fn input_height(available_width: u16, tui_state: &TuiAppState) -> u16 {
    wrapped_input_lines(available_width, tui_state)
        .len()
        .max(1)
        .min(u16::MAX as usize) as u16
}

pub fn composer_height(total_height: u16, available_width: u16, tui_state: &TuiAppState) -> u16 {
    let input_rows = input_height(available_width, tui_state);
    let bottom_height = if tui_state.slash_mode {
        SLASH_COMMANDS.len() as u16
    } else {
        1
    };
    let reserved_rows = 1u16
        .saturating_add(1)
        .saturating_add(bottom_height)
        .saturating_add(1);
    let max_input_rows = total_height.saturating_sub(reserved_rows).max(1);

    input_rows.min(max_input_rows)
}

fn wrapped_input_lines(available_width: u16, tui_state: &TuiAppState) -> Vec<Line<'static>> {
    let prefix = if tui_state.memory_edit_mode {
        "edit>"
    } else {
        tui_state.assistant_state.input_prefix()
    };
    let continuation_prefix = " ".repeat(prefix.chars().count() + 1);
    let first_prefix = format!("{} ", prefix);
    let cursor = Span::styled(
        "█",
        Style::default()
            .fg(Color::White)
            .bg(Color::Gray)
            .add_modifier(Modifier::SLOW_BLINK),
    );
    let width = available_width.max(1) as usize;

    if tui_state.input.is_empty() {
        return vec![Line::from(vec![
            Span::styled(first_prefix, color::dim()),
            cursor,
            Span::raw(" "),
            Span::styled(
                if tui_state.memory_edit_mode {
                    "Type JSON value"
                } else {
                    "Type your message"
                },
                color::dim(),
            ),
        ])];
    }

    let logical_lines: Vec<&str> = tui_state.input.split('\n').collect();
    let last_index = logical_lines.len().saturating_sub(1);
    let mut visual_lines = Vec::new();

    for (index, line) in logical_lines.into_iter().enumerate() {
        let is_first_logical_line = index == 0;
        let is_last_logical_line = index == last_index;
        let mut content = line.to_string();
        if is_last_logical_line {
            content.push(CURSOR_SENTINEL);
        }

        let prefix_text = if is_first_logical_line {
            first_prefix.clone()
        } else {
            continuation_prefix.clone()
        };
        let wrapped_chunks = wrap_input_chunks(&content, width, prefix_text.chars().count());

        for (chunk_index, chunk) in wrapped_chunks.into_iter().enumerate() {
            let current_prefix = if is_first_logical_line && chunk_index == 0 {
                prefix_text.clone()
            } else {
                continuation_prefix.clone()
            };
            visual_lines.push(line_from_chunk(current_prefix, chunk, &cursor));
        }
    }

    visual_lines
}

fn wrap_input_chunks(content: &str, width: usize, prefix_width: usize) -> Vec<String> {
    let content_width = width.saturating_sub(prefix_width).max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for ch in content.chars() {
        if current_width >= content_width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += 1;
    }

    if current.is_empty() {
        chunks.push(String::new());
    } else {
        chunks.push(current);
    }

    chunks
}

fn line_from_chunk(prefix: String, chunk: String, cursor: &Span<'static>) -> Line<'static> {
    let has_cursor = chunk.contains(CURSOR_SENTINEL);
    let text = chunk.replace(CURSOR_SENTINEL, "");
    let mut spans = vec![Span::styled(prefix, color::dim())];

    if !text.is_empty() {
        spans.push(Span::styled(text, color::bright()));
    }

    if has_cursor {
        spans.push(cursor.clone());
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shadow_core::model::AssistantState;

    #[test]
    fn input_height_grows_for_wrapped_text() {
        let mut state = TuiAppState::default();
        state.assistant_state = AssistantState::Idle;
        state.input = "this message should wrap".to_string();

        assert!(input_height(10, &state) > 1);
    }

    #[test]
    fn input_height_grows_for_explicit_newlines() {
        let mut state = TuiAppState::default();
        state.assistant_state = AssistantState::Idle;
        state.input = "first line\nsecond line".to_string();

        assert!(input_height(80, &state) >= 2);
    }

    #[test]
    fn input_height_grows_past_four_lines() {
        let mut state = TuiAppState::default();
        state.assistant_state = AssistantState::Idle;
        state.input = "1234567890 1234567890 1234567890 1234567890 1234567890".to_string();

        assert!(input_height(12, &state) > 4);
    }

    #[test]
    fn composer_height_leaves_room_for_chat() {
        let mut state = TuiAppState::default();
        state.assistant_state = AssistantState::Idle;
        state.input = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6".to_string();

        assert_eq!(composer_height(8, 80, &state), 4);
    }
}
