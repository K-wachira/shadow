use crate::tui::TuiAppState;
use crate::tui::SLASH_COMMANDS;
use crate::tui::SlashCommand;
use crate::tui::bright;
use crate::tui::dim;
use crate::tui::selected_item_style;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use shadow_core::engine::ShadowEngine;


pub fn render_bottom_pane(f: &mut Frame, area: Rect, tui_state: &TuiAppState, shadow_engine: &mut ShadowEngine ) {
    if tui_state.slash_mode {
        render_slash_picker(f, area, tui_state);
        return;
    }

    // normal statusbar
    let left = format!("~ {}", shadow_engine.session_name);

    let right = format!("{}  100% left", &shadow_engine.llm_client.model_name);
    let padding = area.width.saturating_sub((left.len() + right.len()) as u16);

    let line = Line::from(vec![
        Span::styled(left, dim()),
        Span::raw(" ".repeat(padding as usize)),
        Span::styled(right, bright()),
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
        let line = Line::from(vec![Span::styled("  no matching commands", dim())]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let lines: Vec<Line> = matching
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let selected = i == tui_state.slash_cursor;
            let style = if selected {
                selected_item_style()
            } else {
                Style::default().fg(Color::Rgb(153, 153, 153)).add_modifier(Modifier::BOLD)
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(cmd.name, style),
                Span::raw("  "),
                Span::styled(cmd.description, style),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}