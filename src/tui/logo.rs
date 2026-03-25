use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn logo_lines() -> Vec<Line<'static>> {
    let border_color = Style::default().fg(Color::Rgb(70, 70, 80));
    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let hint_style = Style::default().fg(Color::Rgb(80, 80, 180));
    let title_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let version_style = Style::default().fg(Color::DarkGray);

    let w = 50; // inner width of the box (excluding border chars)

    // ┌──────────────────────────────────────────┐
    let top = format!("┌{}┐", "─".repeat(w + 1));
    let bottom = format!("└{}┘", "─".repeat(w + 1));
    let empty = format!("│{:w$}│", "", w = w + 1);

    // │ >_ Shadow (v0.1.0)                       │
    let title_content = ">_ Shadow";
    let version = "(v0.1.0)";
    let title_line = format!(" {}  {}", title_content, version);
    let title_padded = format!("│{:<w$}│", title_line, w = w);

    // │ model:     llama3.2                       │
    let model_line = format!(" {:<10} {}", "model:", "llama3.2");
    let model_padded = format!("│{:<w$}│", model_line, w = w);

    // │ location:  Nairobi                        │
    let loc_line = format!(" {:<10} {}", "location:", "Nairobi");
    let loc_padded = format!("│{:<w$}│", loc_line, w = w);

    vec![
        Line::raw(""),
        // ┌──────────────────────────────────────────┐
        Line::from(Span::styled(top, border_color)),
        // │ >_ Shadow (v0.1.0)                       │
        Line::from(vec![
            Span::styled("│".to_string(), border_color),
            Span::raw("  "),
            Span::styled(">_".to_string(), hint_style),
            Span::raw(" "),
            Span::styled("Shadow".to_string(), title_style),
            Span::raw("  "),
            Span::styled(version.to_string(), version_style),
            Span::styled(
                format!("{:>w$}│", "", w = w - " >_ Shadow  (v0.1.0)".len()),
                border_color,
            ),
        ]),
        // │                                          │
        Line::from(Span::styled(empty.clone(), border_color)),
        // │ model:     llama3.2   /model to change   │
        Line::from(vec![
            Span::styled("│".to_string(), border_color),
            Span::raw("  "),
            Span::styled(format!("{:<10}", "model:"), label_style),
            Span::raw(" "),
            Span::styled("llama3.2".to_string(), value_style),
            Span::raw("   "),
            Span::styled("/model".to_string(), hint_style),
            Span::styled(" to change", label_style),
            Span::styled(
                format!(
                    "{:>w$}│",
                    "",
                    w = w - " model:     llama3.2   /model to change".len()
                ),
                border_color,
            ),
        ]),
        // │ location:  Nairobi                       │
        Line::from(vec![
            Span::styled("│".to_string(), border_color),
            Span::raw("  "),
            Span::styled(format!("{:<10}", "location:"), label_style),
            Span::raw(" "),
            Span::styled("Nairobi".to_string(), value_style),
            Span::styled(
                format!("{:>w$}│", "", w = w - " location:  Nairobi".len()),
                border_color,
            ),
        ]),
        // └──────────────────────────────────────────┘
        Line::from(Span::styled(bottom, border_color)),
        Line::raw(""),
    ]
}