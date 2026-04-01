use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use shadow_core::json_tree::JsonTree;
use shadow_core::json_tree::RowDisplay;

pub struct MemoryTreeWidget {
    pub focused: bool,
    pub max_height: u16,
}

impl StatefulWidget for MemoryTreeWidget {
    type State = JsonTree;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let viewport_height = self.max_height.min(area.height) as usize;
        state.adjust_scroll(viewport_height);

        // ── Header ────────────────────────────────────────────────────────────
        let header_style = if self.focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let focus_hint = if self.focused {
            " (j/k navigate · Enter toggle · y copy · Esc exit)"
        } else {
            " (Enter to focus)"
        };
        let header = Line::from(vec![
            Span::styled("shadow.mind", header_style),
            Span::styled(focus_hint, Style::default().fg(Color::DarkGray)),
        ]);
        buf.set_line(area.left(), area.top(), &header, area.width);

        let tree_top = area.top() + 1;
        let available_width = area.width as usize;

        // ── Render visible rows with wrapping ─────────────────────────────────
        let mut screen_y = tree_top;

        let visible: Vec<_> = state
            .flat
            .iter()
            .enumerate()
            .skip(state.scroll)
            .collect();

        for (idx, row) in visible {
            if screen_y >= area.top() + area.height {
                break;
            }

            let is_cursor = self.focused && idx == state.cursor;
            let indent = "  ".repeat(row.depth);

            let (prefix, key_style, value_text): (&str, Style, String) = match &row.display {
                RowDisplay::Expandable { expanded, child_count, is_object } => {
                    let arrow = if *expanded { "▾ " } else { "▸ " };
                    let brackets = if *is_object {
                        format!("{{ {} }}", child_count)
                    } else {
                        format!("[ {} ]", child_count)
                    };
                    (
                        arrow,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        brackets,
                    )
                }
                RowDisplay::Leaf(val) => (
                    "  ",
                    Style::default().fg(Color::Blue),
                    val.clone(),
                ),
            };

            let value_color = match &row.display {
                RowDisplay::Leaf(val) => leaf_color(val),
                _ => Color::DarkGray,
            };

            let line = Line::from(vec![
                Span::raw(format!("{}{}", indent, prefix)),
                Span::styled(format!("{}: ", row.key), key_style),
                Span::styled(value_text, Style::default().fg(value_color)),
            ]);

            // Measure total char width to calculate how many screen rows needed
            let line_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            let row_count = ((line_width.saturating_sub(1)) / available_width.max(1) + 1) as u16;
            let remaining = area.top() + area.height - screen_y;
            let row_count = row_count.min(remaining);

            let row_rect = Rect::new(area.left(), screen_y, area.width, row_count);

            if is_cursor {
                buf.set_style(row_rect, Style::default().bg(Color::Rgb(35, 35, 55)));
            }

            Paragraph::new(line)
                .wrap(Wrap { trim: false })
                .render(row_rect, buf);

            screen_y += row_count;
        }

        // ── Scroll indicator ──────────────────────────────────────────────────
        if state.flat.len() > viewport_height {
            let pct = (state.scroll * 100)
                / state.flat.len().saturating_sub(viewport_height).max(1);
            let hint = format!("{}%", pct.min(100));
            buf.set_string(
                area.right().saturating_sub(hint.len() as u16 + 1),
                area.top(),
                &hint,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

fn leaf_color(val: &str) -> Color {
    if val.starts_with('"') {
        Color::Green
    } else if val == "true" || val == "false" {
        Color::Magenta
    } else if val == "null" {
        Color::DarkGray
    } else {
        Color::Cyan
    }
}

/// How many screen lines this tree will occupy (header + rows, capped).
pub fn tree_render_height(tree: &JsonTree, max_height: u16) -> u16 {
    (tree.flat.len() as u16 * 3 + 1).min(max_height)
}
