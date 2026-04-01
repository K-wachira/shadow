use crate::tui::TuiAppState;
use crossterm::event::MouseEvent;

pub fn handle_mouse(mouse: MouseEvent, app_state: &mut TuiAppState) {
    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
            app_state.auto_scroll = false;
            app_state.scroll_offset = app_state.scroll_offset.saturating_add(1);
        }
        crossterm::event::MouseEventKind::ScrollDown => {
            app_state.scroll_offset = app_state.scroll_offset.saturating_sub(1);
            if app_state.scroll_offset == 0 {
                app_state.auto_scroll = true;
            }
        }
        _ => {}
    }
}
