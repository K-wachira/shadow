use crate::tui::TuiAppState;
use crate::tui::composer_height;
use crate::tui::render_bottom_pane;
use crate::tui::render_chat;
use crate::tui::render_input;
use crate::tui::render_status_line;

use crate::tui::SLASH_COMMANDS;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use shadow_core::locus::Locus;
// ─── Root render ─────────────────────────────────────────────────────────────

/// Call once per frame from your event loop.
pub fn render(f: &mut Frame, tui_state: &TuiAppState, locus: &mut Locus) {
    // Layout — fixed rows from bottom, history fills the rest:
    //
    //  ┌──────────────────────────────────┐
    //  │  history  (Min, grows to fill)   │
    //  ├──────────────────────────────────┤
    //  │  status line  (1 row)            │  "Choosing… (esc, 4s)"  or blank
    //  ├──────────────────────────────────┤
    //  │  yolo hint    (1 row)            │  right-aligned
    //  ├──────────────────────────────────┤
    //  │  input        (wraps as needed)  │  "> █ Type your message…"
    //  ├──────────────────────────────────┤
    //  │  statusbar    (1 row)            │  "~ [session]   model $0.00 99%"
    //  └──────────────────────────────────┘

    let bottom_height = if tui_state.slash_mode {
        SLASH_COMMANDS.len() as u16
    } else {
        1
    };
    let input_height = composer_height(f.area().height, f.area().width, tui_state);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(input_height),
            Constraint::Length(bottom_height),
        ])
        .split(f.area());

    render_chat(f, chunks[0], tui_state, locus);
    render_status_line(f, chunks[1], tui_state);
    // render_yolo_hint(f, chunks[2], tui_state);
    render_input(f, chunks[3], tui_state);
    render_bottom_pane(f, chunks[4], tui_state, locus);
}
