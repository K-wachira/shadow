use crate::tui::TuiAppState;
use crate::tui::render;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use ratatui::widgets::Block;
use ratatui_textarea::TextArea;
use shadow_core::engine::ShadowEngine;
use shadow_core::model::AssistantState;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub async fn run(
    mut terminal: DefaultTerminal,
    shadow_engine: &mut ShadowEngine,
) -> color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<()>();
    let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();

    let mut app_state = TuiAppState::default();
    let mut input_buf = String::new();
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    let mut stream_start: Option<Instant> = None;

    let mut textarea = TextArea::default();
    textarea.set_block(Block::bordered().title(" input "));

    loop {
        while let Ok(chunk) = rx.try_recv() {
            let chunk = chunk.replace("\\n", "\n"); // clean here
            match shadow_engine.messages.last_mut() {
                Some(Message {
                    kind: MessageKind::AssistantText { text },
                    ..
                }) => {
                    text.push_str(&chunk);
                }
                _ => shadow_engine.messages.push(Message::agent(chunk)),
            }
            if app_state.auto_scroll {
                app_state.scroll_offset = 0;
            }
        }

        if done_rx.try_recv().is_ok() {
            if let Some(Message {
                kind: MessageKind::AssistantText { text },
                ..
            }) = shadow_engine.messages.last()
            {
                shadow_engine.on_stream_complete(&text.clone(), title_tx.clone()).await?;
            }

            app_state.assistant_state = AssistantState::Idle;
            stream_start = None;
        }
        
        if let Ok(title) = title_rx.try_recv() {
            shadow_engine.session_name = title.clone();
            shadow_engine.db.update_session_title(shadow_engine.session_id, &title)?;
        }

        if last_tick.elapsed() >= tick_rate {
            app_state.tick = app_state.tick.wrapping_add(1);
            if let Some(start) = stream_start {
                let secs = start.elapsed().as_secs();
                app_state.assistant_state = AssistantState::Thinking { secs };
            }
            last_tick = Instant::now();
        }

        app_state.input = input_buf.clone();
        app_state.cursor_pos = input_buf.chars().count();

        terminal.draw(|f| render(f, &app_state, shadow_engine))?;

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    if app_state.slash_mode {
                        app_state.slash_mode = false;
                        app_state.slash_input = String::new();
                        input_buf.clear();
                    } else if app_state.history_mode {
                        app_state.history_mode = false;
                        app_state.history_sessions = vec![];
                    } else {
                        break;
                    }
                }

                KeyCode::Char('/') if input_buf.is_empty() => {
                    // only enter slash mode if input is empty
                    app_state.slash_mode = true;
                    app_state.slash_input = String::new();
                    input_buf.push('/');
                }

                KeyCode::Enter => {
                    if app_state.history_mode {
                        // handle session selection — step 5
                        let selected = &app_state.history_sessions[app_state.history_cursor];
                        let selected_id = selected.id;
                        let selected_model = selected
                            .model
                            .clone()
                            .unwrap_or_else(|| app_state.model.clone());
                        let selected_title = selected.title.clone();

                        match shadow_engine.load_session(selected_id) {
                            Ok(messages) => {
                                // clear current conversation

                                shadow_engine.messages.clear();
                                shadow_engine.messages.push(Message::logo());

                                // load messages back into conversation
                                for msg in messages {
                                    match msg.role.as_str() {
                                        "user" => {
                                            shadow_engine.messages.push(Message::user(msg.content))
                                        }
                                        "assistant" => {
                                            shadow_engine.messages.push(Message::agent(msg.content))
                                        }
                                        _ => {}
                                    }
                                }

                                // update session state
                                shadow_engine.session_id = selected_id;
                                shadow_engine.session_name = selected_title.clone();
                                shadow_engine.model = selected_model.clone();

                                // exit history mode, scroll to bottom
                                app_state.history_mode = false;
                                app_state.history_sessions = vec![];
                                app_state.history_cursor = 0;
                                app_state.auto_scroll = true;
                                app_state.scroll_offset = 0;
                            }
                            Err(_) => {
                                app_state.history_mode = false;
                            }
                        }
                    } else if app_state.slash_mode {
                        let cmd = input_buf.trim().to_string();
                        app_state.slash_mode = false;
                        app_state.slash_input = String::new();
                        input_buf.clear();

                        if cmd == "/history" {
                            match shadow_engine.list_sessions(30) {
                                Ok(sessions) => {
                                    app_state.history_sessions = sessions;
                                    app_state.history_mode = true;
                                    app_state.history_cursor = 0;
                                }
                                Err(_) => {}
                            }
                        }
                    } else {
                        let prompt = input_buf.trim().to_string();
                        if prompt.is_empty() {
                            continue;
                        }

                        input_buf.clear();
                        stream_start = Some(Instant::now());

                        let tx = tx.clone();
                        let done_tx = done_tx.clone();

                        let stream_result = shadow_engine.send_message(&prompt).await;

                        match stream_result {
                            Ok(stream) => {
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
                }

                KeyCode::Backspace => {
                    input_buf.pop();
                    if app_state.slash_mode {
                        if input_buf.is_empty() {
                            app_state.slash_mode = false;
                            app_state.slash_input = String::new();
                        } else {
                            app_state.slash_input = input_buf.clone();
                        }
                    }
                }

                KeyCode::Up => {
                    if app_state.history_mode {
                        app_state.history_cursor = app_state.history_cursor.saturating_sub(1);
                    } else {
                        app_state.auto_scroll = false;
                        app_state.scroll_offset = app_state.scroll_offset.saturating_add(1);
                    }
                }

                KeyCode::Down => {
                    if app_state.history_mode {
                        let max = app_state.history_sessions.len().saturating_sub(1);
                        app_state.history_cursor = (app_state.history_cursor + 1).min(max);
                    } else {
                        app_state.scroll_offset = app_state.scroll_offset.saturating_sub(1);
                        if app_state.scroll_offset == 0 {
                            app_state.auto_scroll = true;
                        }
                    }
                }

                KeyCode::Char(c) => {
                    input_buf.push(c);
                    if app_state.slash_mode {
                        app_state.slash_input = input_buf.clone();
                    }
                }
                _ => {}
            },

            Event::Resize(_, _) => {
                // reset scroll to bottom when terminal resizes
                app_state.scroll_offset = 0;
            }

            Event::Mouse(mouse) => match mouse.kind {
                crossterm::event::MouseEventKind::ScrollUp => {
                    app_state.scroll_offset = app_state.scroll_offset.saturating_add(1);
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    app_state.scroll_offset = app_state.scroll_offset.saturating_sub(1);
                }
                _ => {}
            },
            _ => {}
        }
    }

    let _ = shadow_engine.end_session();
    app_state.assistant_state = AssistantState::Idle;
    Ok(())
}
