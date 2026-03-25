use crate::tui::AppState;
use crate::tui::SLASH_COMMANDS;
use crate::tui::SlashCommand;
use crate::tui::bright;
use crate::tui::dim;
use ratatui::{
    Frame,
    layout:: Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_bottom_pane(f: &mut Frame, area: Rect, state: &AppState) {
    if state.slash_mode {
        render_slash_picker(f, area, state);
        return;
    }

    // normal statusbar
    let left = match &state.session_name {
        Some(name) => format!("~ [{}]", name),
        None => "~".to_string(),
    };

    let right = format!("{}  100% left", state.model);
    let padding = area.width.saturating_sub((left.len() + right.len()) as u16);

    let line = Line::from(vec![
        Span::styled(left, dim()),
        Span::raw(" ".repeat(padding as usize)),
        Span::styled(right, bright()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn render_slash_picker(f: &mut Frame, area: Rect, state: &AppState) {
    // filter commands by what's been typed
    let input = state.slash_input.trim_start_matches('/').to_lowercase();

    let matching: Vec<&SlashCommand> = SLASH_COMMANDS
        .iter()
        .filter(|cmd| cmd.name.trim_start_matches('/').starts_with(&input))
        .collect();

    if matching.is_empty() {
        let line = Line::from(vec![Span::styled("  no matching commands", dim())]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    // for now just show the first match — expand to a list later
    let cmd = matching[0];
    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            cmd.name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(cmd.description, dim()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}