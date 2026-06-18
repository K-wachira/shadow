use crate::tui::TuiAppState;
use crossterm::event::KeyCode;
use shadow_core::locus::Locus;
use shadow_core::model::Message;

pub fn handle_key_history(
    key: KeyCode, app_state: &mut TuiAppState, locus: &mut Locus,
) -> color_eyre::Result<bool> {
    match key {
        KeyCode::Esc => {
            app_state.history_mode = false;
            app_state.history_sessions = vec![];
            app_state.reset_persisted_chat();
        }
        KeyCode::Enter => {
            let selected = &app_state.history_sessions[app_state.history_cursor];
            let selected_id = selected.id;
            let selected_title = selected.title.clone();

            match locus.load_session(selected_id) {
                Ok(messages) => {
                    locus.messages.clear();
                    locus
                        .messages
                        .push(Message::logo(&locus.llm_client.model_name));
                    for message in messages {
                        match message.role.as_str() {
                            "user" => locus.messages.push(Message::user(message.content)),
                            "assistant" => locus.messages.push(Message::agent(message.content)),
                            _ => {}
                        }
                    }
                    locus.session_id = selected_id;
                    locus.session_name = selected_title;
                    app_state.history_mode = false;
                    app_state.history_sessions = vec![];
                    app_state.history_cursor = 0;
                    app_state.auto_scroll = true;
                    app_state.scroll_offset = 0;
                    app_state.reset_persisted_chat();
                }
                Err(_) => {
                    app_state.history_mode = false;
                    app_state.reset_persisted_chat();
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app_state.history_cursor = app_state.history_cursor.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app_state.history_sessions.len().saturating_sub(1);
            app_state.history_cursor = (app_state.history_cursor + 1).min(max);
        }
        KeyCode::Delete | KeyCode::Char('d') => {
            if let Some(selected) = app_state.history_sessions.get(app_state.history_cursor) {
                let selected_id = selected.id;
                locus.delete_session(selected_id)?;
                app_state.history_sessions.remove(app_state.history_cursor);
                let max = app_state.history_sessions.len().saturating_sub(1);
                app_state.history_cursor = app_state.history_cursor.min(max);
                app_state.reset_persisted_chat();
            }
        }
        _ => {}
    }
    Ok(false)
}
