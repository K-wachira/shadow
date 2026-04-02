use crate::tui::default_item_style;
use crate::tui::selected_item_style;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use shadow_core::json_tree::FlatRow;
use shadow_core::json_tree::JsonTree;
use shadow_core::json_tree::RowDisplay;

fn row_screen_lines(row: &FlatRow, available_width: usize) -> usize {
    let width = row_content_width(row);
    (width.saturating_sub(1) / available_width.max(1) + 1).max(1)
}

fn row_content_width(row: &FlatRow) -> usize {
    match &row.display {
        RowDisplay::Leaf(val) => {
            row.depth * 2 + 2 + row.key.chars().count() + 2 + val.chars().count()
        }
        RowDisplay::Expandable {
            child_count,
            is_object,
            ..
        } => {
            let bracket = if *is_object {
                format!("└ {}", child_count)
            } else {
                format!("⌑ {}", child_count)
            };
            row.depth * 2 + 2 + row.key.chars().count() + 2 + bracket.chars().count()
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
    let rows: u16 = tree
        .flat
        .iter()
        .map(|row| row_screen_lines(row, available_width.max(1) as usize) as u16)
        .sum();

    header + rows
}

pub fn tree_cursor_screen_row(tree: &JsonTree, available_width: u16) -> usize {
    let width = available_width.max(1) as usize;
    1 + tree
        .flat
        .iter()
        .take(tree.cursor)
        .map(|row| row_screen_lines(row, width))
        .sum::<usize>()
}

pub fn tree_to_lines(tree: &JsonTree, focused: bool, editing: bool) -> Vec<Line<'static>> {
    let header_style = if focused {
        Style::default()
            .fg(Color::Rgb(215, 119, 87))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let focus_hint = if focused {
        if editing {
            " (edit mode · Enter save · Esc cancel)"
        } else {
            " (j/k navigate · Enter toggle · e edit · y copy · Esc exit)"
        }
    } else {
        " (/memory to view memory)"
    };

    let mut lines = vec![Line::from(vec![
        Span::styled("shadow.mind", header_style),
        Span::styled(focus_hint, Style::default().fg(Color::DarkGray)),
    ])];

    for (idx, row) in tree.flat.iter().enumerate() {
        let is_cursor = focused && idx == tree.cursor;
        let indent = "  ".repeat(row.depth);
        let base_style = if is_cursor {
            selected_item_style()
        } else {
            default_item_style()
        };

        let (prefix, value_text): (&str, String) = match &row.display {
            RowDisplay::Expandable {
                expanded,
                child_count,
                is_object,
            } => {
                let arrow = if *expanded { "▾ " } else { "▸ " };
                let brackets = if *is_object {
                    format!("└ {}", child_count)
                } else {
                    format!("⌑ {}", child_count)
                };
                (arrow, brackets)
            }
            RowDisplay::Leaf(val) => ("  ", val.clone()),
        };

        let value_style = if is_cursor {
            base_style
        } else {
            Style::default().fg(match &row.display {
                RowDisplay::Leaf(val) => leaf_color(val),
                _ => Color::DarkGray,
            })
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}{}", indent, prefix), base_style),
            Span::styled(format!("{}: ", row.key), base_style),
            Span::styled(value_text, value_style),
        ]));
    }

    lines
}
