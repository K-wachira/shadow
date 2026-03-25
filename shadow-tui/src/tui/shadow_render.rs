use crate::tui::TuiAppState;
use crate::tui::render_bottom_pane;
use crate::tui::render_chat;
use crate::tui::render_input;
use crate::tui::render_status_line;
use crate::tui::render_yolo_hint;

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use shadow_core::engine::ShadowEngine;

// ─── Root render ─────────────────────────────────────────────────────────────

/// Call once per frame from your event loop.
pub fn render(f: &mut Frame, tui_state: &TuiAppState,   shadow_engine: &mut ShadowEngine) {
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

    render_chat(f, chunks[0], tui_state, shadow_engine);
    render_status_line(f, chunks[1], tui_state);
    render_yolo_hint(f, chunks[2], tui_state);
    render_input(f, chunks[3], tui_state);
    render_bottom_pane(f, chunks[4], tui_state);
}