use crate::tui::AppState;
use crate::tui::dim;
use ratatui::{
    Frame,
    layout:: Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
// ─── Status line ─────────────────────────────────────────────────────────────
pub fn render_status_line(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(text) = state.assistant_state.status_text() {
        let spinner = state.assistant_state.spinner(state.tick);
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

pub fn render_yolo_hint(f: &mut Frame, area: Rect, state: &AppState) {
    let (label, color) = if state.yolo_mode {
        ("YOLO Mode", Color::Red)
    } else {
        ("Safe YOLO", Color::Magenta)
    };

    let suffix = "  (ctrl + y to toggle)";
    let full_len = (label.len() + suffix.len()) as u16;
    let x = area.x + area.width.saturating_sub(full_len + 1);
    let right = Rect {
        x,
        y: area.y,
        width: full_len + 1,
        height: 1,
    };

    let line = Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(color)),
        Span::styled(suffix.to_string(), dim()),
    ]);
    f.render_widget(Paragraph::new(line), right);
}