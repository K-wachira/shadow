use crate::tui::AppState;
use crate::tui::render_bottom_pane;
use crate::tui::render_chat;
use crate::tui::render_input;
use crate::tui::render_status_line;
use crate::tui::render_yolo_hint;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};


// ─── Root render ─────────────────────────────────────────────────────────────

/// Call once per frame from your event loop.
pub fn render(f: &mut Frame, state: &AppState) {
    // Layout — fixed rows from bottom, history fills the rest:
    //
    //  ┌──────────────────────────────────┐
    //  │  history  (Min, grows to fill)   │
    //  ├──────────────────────────────────┤
    //  │  status line  (1 row)            │  "Choosing… (esc, 4s)"  or blank
    //  ├──────────────────────────────────┤
    //  │  yolo hint    (1 row)            │  right-aligned
    //  ├──────────────────────────────────┤
    //  │  input        (1 row)            │  "> █ Type your message…"
    //  ├──────────────────────────────────┤
    //  │  statusbar    (1 row)            │  "~ [session]   model $0.00 99%"
    //  └──────────────────────────────────┘

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_chat(f, chunks[0], state);
    render_status_line(f, chunks[1], state);
    render_yolo_hint(f, chunks[2], state);
    render_input(f, chunks[3], state);
    render_bottom_pane(f, chunks[4], state);
}