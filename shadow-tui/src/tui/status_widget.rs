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

// ─── Status line ─────────────────────────────────────────────────────────────
pub fn render_status_line(f: &mut Frame, area: Rect, tui_state: &TuiAppState) {
    if let Some(confirm) = &tui_state.pending_confirm {
        let line = Line::from(vec![
            Span::styled(
                "⚠  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(confirm.prompt.clone(), Style::default().fg(Color::Yellow)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    if let Some(text) = tui_state.assistant_state.status_text() {
        let spinner = tui_state.assistant_state.spinner(tui_state.tick);
        let line = Line::from(vec![
            Span::styled(format!("{}  ", spinner), Style::default().fg(Color::Yellow)),
            Span::styled(
                text,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
    }
    // Idle: nothing — blank row
}

#[allow(dead_code)]
pub fn render_yolo_hint(f: &mut Frame, area: Rect, locus: &Locus) {
    let suffix = locus.ephemeral.clone().unwrap_or_default();
    let full_len = (suffix.len()) as u16;
    let x = area.x + area.width.saturating_sub(full_len + 1);
    let right = Rect {
        x,
        y: area.y,
        width: full_len + 1,
        height: 1,
    };

    let line = Line::from(vec![
        // Span::styled(label.to_string(), Style::default()),
        Span::styled(suffix.to_string(), color::dim()),
    ]);
    f.render_widget(Paragraph::new(line), right);
}
