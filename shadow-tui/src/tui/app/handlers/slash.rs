use crate::tui::PendingConfirm;
use crate::tui::PendingConfirmAction;
use crate::tui::SLASH_COMMANDS;
use crate::tui::SlashCommand;
use crate::tui::TuiAppState;
use crate::tui::ensure_memory_cursor_visible;
use crate::tui::tui_models::ActiveOperation;
use crossterm::event::KeyCode;
use json5::from_str;
use shadow_core::engine::ShadowEngine;
use shadow_core::json_tree::JsonTree;
use shadow_continuity::mind::ShadowMind;
use shadow_core::model::Message;
use shadow_core::model::ToolCall;
use shadow_core::model::ToolPayload;
use shadow_utils::utils::format_timestamp;
use std::fs::read_to_string;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
#[derive(Debug)]
enum SlashAction {
    New,
    Delete,
    History,
    Ingest,
    Reflect,
    Rename,
    Exit,
    Memory,
    Unknown,
}

impl SlashAction {
    fn parse(input: &str) -> Self {
        match input.trim() {
            "/delete" => Self::Delete,
            "/new" => Self::New,
            "/ingest" => Self::Ingest,
            "/reflect" => Self::Reflect,
            "/rename" => Self::Rename,
            "/exit" => Self::Exit,
            "/history" => Self::History,
            "/memory" => Self::Memory,
            _ => Self::Unknown,
        }
    }
}

pub async fn handle_key_slash(
    key: KeyCode, app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
    reflect_tx: mpsc::UnboundedSender<ShadowMind>,
) -> color_eyre::Result<bool> {
    let max = SLASH_COMMANDS.len().saturating_sub(1);
    match key {
        KeyCode::Esc => handle_escape(app_state, input_buf),
        KeyCode::Enter => {
            return handle_enter(app_state, engine, input_buf, reflect_tx).await;
        }
        KeyCode::Backspace => handle_backspace(app_state, input_buf),
        KeyCode::Up => {
            if app_state.slash_cursor == 0 {
                app_state.slash_cursor = max
            } else {
                app_state.slash_cursor = app_state.slash_cursor.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if app_state.slash_cursor == max {
                app_state.slash_cursor = 0;
            } else {
                app_state.slash_cursor = (app_state.slash_cursor + 1).min(max);
            }
        }
        KeyCode::Char(c) => {
            input_buf.push(c);
            app_state.slash_input = input_buf.clone();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_escape(app_state: &mut TuiAppState, input_buf: &mut String) {
    app_state.slash_mode = false;
    app_state.slash_input = String::new();
    input_buf.clear();
}

fn handle_backspace(app_state: &mut TuiAppState, input_buf: &mut String) {
    input_buf.pop();
    if input_buf.is_empty() {
        app_state.slash_mode = false;
        app_state.slash_input = String::new();
    } else {
        app_state.slash_input = input_buf.clone();
    }
}

async fn handle_enter(
    app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
    reflect_tx: mpsc::UnboundedSender<ShadowMind>,
) -> color_eyre::Result<bool> {
    let command = selected_command(app_state).unwrap_or("");
    let action = SlashAction::parse(command);
    reset_slash_picker(app_state, input_buf);
    if let Some(pending) = pending_confirmation_for(action) {
        app_state.pending_confirm = Some(pending);
        return Ok(false);
    }
    run_action(action, app_state, engine, input_buf, reflect_tx).await
}

pub async fn handle_pending_confirm_key(
    key: KeyCode, app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
    reflect_tx: mpsc::UnboundedSender<ShadowMind>,
) -> color_eyre::Result<bool> {
    let Some(pending) = app_state.pending_confirm.as_ref() else {
        return Ok(false);
    };

    match key {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            let action = action_from_confirm(pending.action);
            app_state.pending_confirm = None;
            run_action(action, app_state, engine, input_buf, reflect_tx).await
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app_state.pending_confirm = None;
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn selected_command(app_state: &TuiAppState) -> Option<&'static str> {
    let input = app_state.slash_input.trim_start_matches('/').to_lowercase();
    let matching: Vec<&SlashCommand> = SLASH_COMMANDS
        .iter()
        .filter(|cmd| cmd.name.trim_start_matches('/').starts_with(&input))
        .collect();
    matching
        .get(app_state.slash_cursor)
        .map(|candidate| candidate.name)
}

fn reset_slash_picker(app_state: &mut TuiAppState, input_buf: &mut String) {
    app_state.slash_mode = false;
    app_state.slash_input = String::new();
    app_state.slash_cursor = 0;
    input_buf.clear();
}

async fn run_action(
    action: SlashAction, app_state: &mut TuiAppState, engine: &mut ShadowEngine,
    input_buf: &mut String, reflect_tx: mpsc::UnboundedSender<ShadowMind>,
) -> color_eyre::Result<bool> {
    match action {
        SlashAction::New => handle_action_new(app_state, engine),
        SlashAction::Delete => handle_action_delete(engine)?,
        SlashAction::History => handle_action_history(app_state, engine),
        SlashAction::Ingest => handle_action_ingest(app_state, engine),
        SlashAction::Reflect => handle_action_reflect(app_state, engine, reflect_tx).await?,
        SlashAction::Rename => handle_action_rename(app_state, engine, input_buf),
        SlashAction::Memory => handle_action_memory(app_state, engine),
        SlashAction::Exit => {
            return Ok(matches!(app_state.active_op, ActiveOperation::Idle));
        }
        SlashAction::Unknown => {}
    }
    Ok(false)
}

fn pending_confirmation_for(action: SlashAction) -> Option<PendingConfirm> {
    match action {
        SlashAction::Delete => Some(PendingConfirm {
            action: PendingConfirmAction::DeleteSession,
            prompt: "Confirm /delete? Enter/y yes · Esc/n no".to_string(),
        }),
        SlashAction::Reflect => Some(PendingConfirm {
            action: PendingConfirmAction::ReflectMind,
            prompt: "Confirm /refect? Enter/y yes · Esc/n no".to_string(),
        }),
        _ => None,
    }
}

fn action_from_confirm(action: PendingConfirmAction) -> SlashAction {
    match action {
        PendingConfirmAction::DeleteSession => SlashAction::Delete,
        PendingConfirmAction::ReflectMind => SlashAction::Reflect,
    }
}

fn handle_action_new(app_state: &mut TuiAppState, engine: &mut ShadowEngine) {
    if engine.messages.len() > 1 {
        engine.start_new_session();
        app_state.auto_scroll = true;
        app_state.scroll_offset = 0;
        app_state.reset_persisted_chat();
    }
}

fn handle_action_delete(engine: &mut ShadowEngine) -> color_eyre::Result<()> {
    engine.delete_current_session()?;
    engine.messages = engine.messages.clone();
    Ok(())
}

fn handle_action_history(app_state: &mut TuiAppState, engine: &mut ShadowEngine) {
    if let Ok(sessions) = engine.list_sessions(30) {
        app_state.history_sessions = sessions;
        app_state.history_mode = true;
        app_state.history_cursor = 0;
        app_state.reset_persisted_chat();
    }
}

fn handle_action_ingest(app_state: &mut TuiAppState, engine: &mut ShadowEngine) {
    app_state.active_op = ActiveOperation::Ingesting(Instant::now());
    match engine.ingest_icloud_logs() {
        Ok(logs) => {
            let mut tool = ToolCall::new("Ingest", "iCloud logs");
            tool.payload = Some(ToolPayload::Logs(logs.clone()));
            tool.finish(
                logs.iter()
                    .map(|log| format!("{} — {}", format_timestamp(&log.time_stamp), log.content))
                    .collect(),
            );
            engine.messages.push(Message::tool(tool));
        }
        Err(e) => tracing::error!("ingest error: {}", e),
    }
    app_state.active_op = ActiveOperation::Idle;
}

async fn handle_action_reflect(
    app_state: &mut TuiAppState,
    engine: &mut ShadowEngine,
    reflect_tx: mpsc::UnboundedSender<ShadowMind>,
) -> color_eyre::Result<()> {
    let logs_json = engine.gather_reflect_input()?;
    let llm_client = Arc::clone(&engine.llm_client);
    let paths = engine.paths.clone();
    let current_mind = engine.mind.clone();

    let token = CancellationToken::new();
    app_state.cancel_token = token.clone();
    app_state.active_op = ActiveOperation::Reflecting(Instant::now());

    tokio::spawn(async move {
        tokio::select! {
            result = ShadowEngine::reflect(llm_client, paths, current_mind, logs_json) => {
                match result {
                    Ok(new_mind) => { let _ = reflect_tx.send(new_mind); }
                    Err(e) => tracing::error!("reflect error: {}", e),
                }
            }
            _ = token.cancelled() => {
                tracing::info!("reflection cancelled");
            }
        }
    });
    Ok(())
}

fn handle_action_rename(
    app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
) {
    app_state.rename_mode = true;
    input_buf.clear();
    input_buf.push_str(engine.session_name.clone().as_str());
}

fn handle_action_memory(app_state: &mut TuiAppState, engine: &mut ShadowEngine) {
    let path = engine.paths.mind.clone();
    let expanded = true;
    match read_to_string(&path) {
        Ok(raw) => match from_str::<serde_json::Value>(&raw) {
            Ok(value) => {
                let tree = JsonTree::from_value(&value, expanded);
                let msg_idx = engine.messages.len();
                engine.messages.push(Message::memory_tree(tree));
                app_state.memory_focus = Some(msg_idx);
                app_state.memory_source_path = Some(path);
                app_state.memory_edit_mode = false;
                app_state.memory_edit_buffer.clear();
                app_state.memory_edit_path = None;
                let _ = ensure_memory_cursor_visible(app_state, engine);
            }
            Err(e) => tracing::error!("failed to parse shadow.mind: {}", e),
        },
        Err(e) => tracing::error!("failed to read shadow.mind: {}", e),
    }
}
