use crate::tui::TuiAppState;
use crossterm::event::KeyCode;

/// Read-only log browser: navigate with ↑↓/jk, Esc to close. Entries live in
/// `app_state.log_entries`, so no `Locus` access is needed here.
pub fn handle_key_logs(key: KeyCode, app_state: &mut TuiAppState) -> color_eyre::Result<bool> {
    match key {
        KeyCode::Esc => {
            app_state.logs_mode = false;
            app_state.log_entries = vec![];
            app_state.logs_cursor = 0;
            app_state.reset_persisted_chat();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app_state.logs_cursor = app_state.logs_cursor.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app_state.log_entries.len().saturating_sub(1);
            app_state.logs_cursor = (app_state.logs_cursor + 1).min(max);
        }
        _ => {}
    }
    Ok(false)
}
