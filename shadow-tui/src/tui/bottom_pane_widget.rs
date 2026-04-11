use crate::tui::SLASH_COMMANDS;
use crate::tui::SlashCommand;
use crate::tui::TuiAppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use shadow_core::locus::Locus;
use shadow_utils::color;

pub fn render_bottom_pane(f: &mut Frame, area: Rect, tui_state: &TuiAppState, locus: &mut Locus) {
    if tui_state.slash_mode {
        render_slash_picker(f, area, tui_state);
        return;
    }

    // normal statusbar
    let left = format!("~ {}", locus.session_name);

    let right = format!("{}  100% left", &locus.llm_client.model_name);
    let padding = area.width.saturating_sub((left.len() + right.len()) as u16);

    let line = Line::from(vec![
        Span::styled(left, color::dim()),
        Span::raw(" ".repeat(padding as usize)),
        Span::styled(right, color::bright()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn render_slash_picker(f: &mut Frame, area: Rect, tui_state: &TuiAppState) {
    let input = tui_state.slash_input.trim_start_matches('/').to_lowercase();
    let matching: Vec<&SlashCommand> = SLASH_COMMANDS
        .iter()
        .filter(|cmd| cmd.name.trim_start_matches('/').starts_with(&input))
        .collect();

    if matching.is_empty() {
        let line = Line::from(vec![Span::styled("  no matching commands", color::dim())]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let (_, max_len) = matching.iter().fold((usize::MAX, 0), |(min, max), cmd| {
        let len = cmd.name.len();
        (min.min(len), max.max(len))
    });

    let width = max_len + 3;
    let lines: Vec<Line> = matching
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let selected = i == tui_state.slash_cursor;
            let style = if selected {
                color::selected_item_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Rgb(153, 153, 153))
                    .add_modifier(Modifier::BOLD)
            };
            let padded_name = format!("{:<width$}", cmd.name, width = width);
            Line::from(vec![
                Span::raw("  "),
                Span::styled(padded_name, style),
                Span::raw("  "),
                Span::styled(cmd.description, style),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}
