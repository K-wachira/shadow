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
use crate::tui::default_item_style;
use crate::tui::selected_item_style;

pub struct MemoryTreeWidget {
    pub focused: bool,
    pub viewport_height: u16, // used for scroll calc, not rect sizing

}

impl StatefulWidget for MemoryTreeWidget {
    type State = JsonTree;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let visible_screen_height = area.height.saturating_sub(1) as usize; // minus header
        state.adjust_scroll(visible_screen_height);

        // ── Header ────────────────────────────────────────────────────────────
        let header_style = if self.focused {
            Style::default().fg(Color::Rgb(215, 119, 87)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let focus_hint = if self.focused {
            " (j/k navigate · Enter toggle · y copy · Esc exit)"
        } else {
            " (/memory to view memory)"
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
            let key_style = if is_cursor { selected_item_style() } else { default_item_style() };

            let (prefix, key_style, value_text): (&str, Style, String) = match &row.display {
                RowDisplay::Expandable { expanded, child_count, is_object } => {
                    let arrow = if *expanded { "▾ " } else { "▸ " };
                    let brackets = if *is_object {
                        format!("└ {}", child_count)
                    } else {
                        format!("⌑ {}", child_count)
                    };
               
                        (arrow, key_style, brackets)
                   
                }
                RowDisplay::Leaf(val) => (
                    "  ",
                    key_style,
                    val.clone(),
                ),
            };

            //Number of children and the emoji
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
                buf.set_style(row_rect, selected_item_style());
            }

            Paragraph::new(line)
                .wrap(Wrap { trim: false })
                .render(row_rect, buf);

            screen_y += row_count;
        }

        // ── Scroll indicator ──────────────────────────────────────────────────
        if state.flat.len() > visible_screen_height {
            let pct = (state.scroll * 100)
                / state.flat.len().saturating_sub(visible_screen_height).max(1);
            let hint = format!("{}%", pct.min(100));
            buf.set_string(
                area.right().saturating_sub(hint.len() as u16 + 1),
                area.top(),
                &hint,
                Style::default().fg(Color::Cyan),
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
pub fn tree_render_height(tree: &JsonTree, available_width: u16) -> u16 {
    let header = 1u16;
    let rows: u16 = tree.flat.iter().map(|row| {
        let content_width = match &row.display {
            RowDisplay::Leaf(val) => {
                row.depth * 2 + 2 + row.key.chars().count() + 2 + val.chars().count()
            }
            RowDisplay::Expandable { child_count, is_object, .. } => {
                let bracket = if *is_object {
                    format!("└ {}", child_count)
                } else {
                    format!("⌑ {}", child_count)
                };
                row.depth * 2 + 2 + row.key.chars().count() + 2 + bracket.chars().count()
            }
        };
        ((content_width.saturating_sub(1)) / available_width.max(1) as usize + 1) as u16
    }).sum();

    header + rows
}
