use crate::tui::TuiAppState;
use crate::tui::ensure_memory_cursor_visible;
use crate::tui::tui_models::ActiveOperation;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use json5::from_str;
use shadow_core::engine::ShadowEngine;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

pub async fn handle_key_normal(
    key: KeyEvent, app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
    tx: mpsc::UnboundedSender<String>, done_tx: mpsc::UnboundedSender<()>,
) -> color_eyre::Result<bool> {
    if let Some(focus_idx) = app_state.memory_focus {
        let mut keep_cursor_visible = false;
        let mut scroll_transcript_up = false;
        let mut scroll_transcript_down = false;
        if let Some(Message {
            kind: MessageKind::MemoryTree(tree),
            ..
        }) = engine.messages.get_mut(focus_idx)
        {
            if app_state.memory_edit_mode {
                match key.code {
                    KeyCode::Esc => {
                        app_state.memory_edit_mode = false;
                        app_state.memory_edit_buffer.clear();
                        app_state.memory_edit_path = None;
                    }
                    KeyCode::Enter => {
                        let parsed =
                            match from_str::<serde_json::Value>(&app_state.memory_edit_buffer) {
                                Ok(value) => value,
                                Err(e) => {
                                    tracing::error!("invalid JSON value: {}", e);
                                    return Ok(false);
                                }
                            };

                        let Some(target_path) = app_state.memory_edit_path.clone() else {
                            tracing::error!("no selected memory row to edit");
                            app_state.memory_edit_mode = false;
                            app_state.memory_edit_buffer.clear();
                            return Ok(false);
                        };

                        let tree_before = tree.clone();
                        if !tree.set_value_at_path(&target_path, &parsed) {
                            tracing::error!("failed to update selected value");
                            return Ok(false);
                        }

                        if let Some(path) = app_state.memory_source_path.clone() {
                            let value = tree.to_value();
                            if let Err(e) = persist_json_value(&path, &value) {
                                *tree = tree_before;
                                tracing::error!("failed to write shadow.mind: {}", e);
                                return Ok(false);
                            }
                        } else {
                            tracing::error!("missing source path for memory file");
                        }

                        app_state.memory_edit_mode = false;
                        app_state.memory_edit_buffer.clear();
                        app_state.memory_edit_path = None;
                    }
                    KeyCode::Backspace => {
                        app_state.memory_edit_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        app_state.memory_edit_buffer.push(c);
                    }
                    _ => {}
                }
                return Ok(false);
            }

            match key.code {
                KeyCode::Esc => {
                    app_state.memory_focus = None;
                    app_state.memory_edit_mode = false;
                    app_state.memory_edit_buffer.clear();
                    app_state.memory_edit_path = None;
                    app_state.memory_source_path = None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if tree.cursor > 0 {
                        tree.move_up();
                        keep_cursor_visible = true;
                    } else {
                        scroll_transcript_up = true;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if tree.cursor + 1 < tree.flat.len() {
                        tree.move_down();
                        keep_cursor_visible = true;
                    } else {
                        scroll_transcript_down = true;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    tree.toggle_current();
                    keep_cursor_visible = true;
                }
                KeyCode::Char('e') => {
                    if let Some(path) = tree.selected_path() {
                        if let Some(current) = tree.selected_leaf_literal() {
                            app_state.memory_edit_mode = true;
                            app_state.memory_edit_path = Some(path);
                            app_state.memory_edit_buffer = current;
                        } else {
                            tracing::info!("select a leaf value to edit");
                        }
                    }
                }
                KeyCode::Char('y') => {
                    if let Some(val) = tree.selected_value() {
                        tracing::debug!("copied: {}", val);
                    }
                }
                _ => {}
            }
        }
        if keep_cursor_visible {
            ensure_memory_cursor_visible(app_state, engine)?;
        } else if scroll_transcript_up {
            app_state.scroll_transcript_up();
        } else if scroll_transcript_down {
            app_state.scroll_transcript_down();
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Esc => {
            app_state.cancel_token.cancel();
            app_state.active_op = ActiveOperation::Idle;
        }

        KeyCode::Enter => {
            if key.modifiers.contains(KeyModifiers::SHIFT) && !app_state.rename_mode {
                input_buf.push('\n');
                return Ok(false);
            }

            let prompt = input_buf.trim().to_string();
            if prompt.is_empty() {
                return Ok(false);
            }
            if app_state.rename_mode {
                match engine.db.update_session_title(engine.session_id, input_buf) {
                    Ok(()) => {
                        engine.session_name = input_buf.clone();
                    }
                    Err(e) => {
                        tracing::error!("Error on title rename: {}", e);
                    }
                }
                input_buf.clear();
                return Ok(false);
            }
            input_buf.clear();

            match engine.send_message(&prompt).await {
                Ok(stream) => {
                    app_state.active_op = ActiveOperation::Streaming(Instant::now());
                    let mut stream = Box::pin(stream);
                    let token = CancellationToken::new();
                    app_state.cancel_token = token.clone();
                    tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                chunk = stream.next() => {
                                    match chunk {
                                        Some(chunk) => { let _ = tx.send(chunk); }
                                        None => break,
                                    }
                                }
                                _ = token.cancelled() => {
                                    tracing::info!("stream cancelled");
                                    break;
                                }
                            }
                        }
                        let _ = done_tx.send(()); // fires on both completion and cancellation
                    });
                }
                Err(e) => tracing::error!("send_message error: {}", e),
            }
        }
        KeyCode::Backspace => {
            input_buf.pop();
        }
        KeyCode::Char('/') if input_buf.is_empty() => {
            app_state.slash_mode = true;
            app_state.slash_input = String::new();
            app_state.slash_cursor = 0;
            input_buf.push('/');
        }
        KeyCode::Char(c) => {
            input_buf.push(c);
        }
        KeyCode::Up => {
            app_state.scroll_transcript_up();
        }
        KeyCode::Down => {
            app_state.scroll_transcript_down();
        }
        _ => {}
    }
    Ok(false)
}

fn persist_json_value(path: &Path, value: &serde_json::Value) -> color_eyre::Result<()> {
    let content = json5::to_string(value)
        .or_else(|_| serde_json::to_string_pretty(value))
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let tmp = temp_path_for(path);
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut tmp = path.to_path_buf();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("shadowmind.json5");
    tmp.set_file_name(format!("{}.tmp", file_name));
    tmp
}
