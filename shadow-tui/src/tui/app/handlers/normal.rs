use crate::tui::TuiAppState;
use crossterm::event::KeyCode;
use json5::from_str;
use shadow_core::engine::ShadowEngine;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub async fn handle_key_normal(
    key: KeyCode, app_state: &mut TuiAppState, engine: &mut ShadowEngine, input_buf: &mut String,
    tx: mpsc::UnboundedSender<String>, done_tx: mpsc::UnboundedSender<()>,
) -> color_eyre::Result<bool> {
    if let Some(focus_idx) = app_state.memory_focus {
        if let Some(Message {
            kind: MessageKind::MemoryTree(tree),
            ..
        }) = engine.messages.get_mut(focus_idx)
        {
            if app_state.memory_edit_mode {
                match key {
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
                                    eprintln!("invalid JSON value: {}", e);
                                    return Ok(false);
                                }
                            };

                        let Some(target_path) = app_state.memory_edit_path.clone() else {
                            eprintln!("no selected memory row to edit");
                            app_state.memory_edit_mode = false;
                            app_state.memory_edit_buffer.clear();
                            return Ok(false);
                        };

                        let tree_before = tree.clone();
                        if !tree.set_value_at_path(&target_path, &parsed) {
                            eprintln!("failed to update selected value");
                            return Ok(false);
                        }

                        if let Some(path) = app_state.memory_source_path.clone() {
                            let value = tree.to_value();
                            if let Err(e) = persist_json_value(&path, &value) {
                                *tree = tree_before;
                                eprintln!("failed to write shadow.mind: {}", e);
                                return Ok(false);
                            }
                        } else {
                            eprintln!("missing source path for memory file");
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

            match key {
                KeyCode::Esc => {
                    app_state.memory_focus = None;
                    app_state.memory_edit_mode = false;
                    app_state.memory_edit_buffer.clear();
                    app_state.memory_edit_path = None;
                    app_state.memory_source_path = None;
                }
                KeyCode::Up | KeyCode::Char('k') => tree.move_up(),
                KeyCode::Down | KeyCode::Char('j') => tree.move_down(),
                KeyCode::Enter | KeyCode::Char(' ') => tree.toggle_current(),
                KeyCode::Char('e') => {
                    if let Some(path) = tree.selected_path() {
                        if let Some(current) = tree.selected_leaf_literal() {
                            app_state.memory_edit_mode = true;
                            app_state.memory_edit_path = Some(path);
                            app_state.memory_edit_buffer = current;
                        } else {
                            eprintln!("select a leaf value to edit");
                        }
                    }
                }
                KeyCode::Char('y') => {
                    if let Some(val) = tree.selected_value() {
                        eprintln!("copied: {}", val);
                    }
                }
                _ => {}
            }
        }
        return Ok(false);
    }

    match key {
        KeyCode::Enter => {
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
                        eprintln!("Error on title rename: {}", e);
                    }
                }
                input_buf.clear();
                return Ok(false);
            }
            input_buf.clear();

            match engine.send_message(&prompt).await {
                Ok(stream) => {
                    app_state.stream_start = Some(Instant::now());
                    let mut stream = Box::pin(stream);
                    tokio::spawn(async move {
                        while let Some(chunk) = stream.next().await {
                            let _ = tx.send(chunk);
                        }
                        let _ = done_tx.send(());
                    });
                }
                Err(_) => {}
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
            app_state.auto_scroll = false;
            app_state.scroll_offset = app_state.scroll_offset.saturating_add(1);
        }
        KeyCode::Down => {
            app_state.scroll_offset = app_state.scroll_offset.saturating_sub(1);
            if app_state.scroll_offset == 0 {
                app_state.auto_scroll = true;
            }
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
