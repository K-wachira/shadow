use crate::tui::TuiAppState;
use crate::tui::ensure_memory_cursor_visible;
use crossterm::event::MouseEvent;
use shadow_core::locus::Locus;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;

pub fn handle_mouse(
    mouse: MouseEvent, app_state: &mut TuiAppState, locus: &mut Locus,
) -> color_eyre::Result<()> {
    if let Some(focus_idx) = app_state.memory_focus {
        if let Some(Message {
            kind: MessageKind::MemoryTree(tree),
            ..
        }) = locus.messages.get_mut(focus_idx)
        {
            match mouse.kind {
                crossterm::event::MouseEventKind::ScrollUp => {
                    if tree.cursor > 0 {
                        tree.move_up();
                        ensure_memory_cursor_visible(app_state, locus)?;
                    } else {
                        app_state.scroll_transcript_up();
                    }
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    if tree.cursor + 1 < tree.flat.len() {
                        tree.move_down();
                        ensure_memory_cursor_visible(app_state, locus)?;
                    } else {
                        app_state.scroll_transcript_down();
                    }
                }
                _ => return Ok(()),
            }
            return Ok(());
        }
    }

    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
            app_state.scroll_transcript_up();
        }
        crossterm::event::MouseEventKind::ScrollDown => {
            app_state.scroll_transcript_down();
        }
        _ => {}
    }
    Ok(())
}
