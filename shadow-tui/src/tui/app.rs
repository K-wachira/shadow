use crate::tui::TuiAppState;
use crate::tui::render;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::MouseEvent;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use shadow_core::engine::ShadowEngine;
use shadow_core::model::AssistantState;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use crate::tui::SLASH_COMMANDS;
use crate::tui::SlashCommand;
use shadow_core::mind::ShadowMind;
use std::sync::Arc;
use shadow_core::mind::gather_reflect_input;
use shadow_core::mind::reflect_with_input;
use tokio::sync::mpsc::error::TryRecvError;
use shadow_core::model::ToolCall;
use shadow_core::model::ToolPayload;
use crate::tui::utils::format_timestamp;

enum SlashAction {
    New,
    Delete,
    History,
    Ingest,
    Reflect,
    Exit,
    Unknown(())
}

impl SlashAction {
    fn parse(input: &str) -> Self {
        match input.trim() {
            "/delete" => Self::Delete,
            "/new" => Self::New,
            "/ingest" => Self::Ingest,
            "/refect" => Self::Reflect,
            "/exit" => Self::Exit,
            "/history" => Self::History,
            _ => Self::Unknown(()),
        }
    }
}

pub async fn run( mut terminal: DefaultTerminal,shadow_engine: &mut ShadowEngine) -> 
color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<()>();
    let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
    let (reflect_tx, mut reflect_rx) = mpsc::unbounded_channel::<ShadowMind>();
    let mut app_state = TuiAppState::default();
    let mut input_buf = String::new();

    loop {
        process_channels(
            &mut rx, &mut done_rx, &mut title_rx,
            &mut app_state, shadow_engine,
            title_tx.clone(),
            &mut reflect_rx,
        ).await?;

        update_tick(&mut app_state);
        update_assistant_state(&mut app_state);

        app_state.input = input_buf.clone();
        app_state.cursor_pos = input_buf.chars().count();

        if let Err(e) = terminal.draw(|f| render(f, &app_state, shadow_engine)) {
            if !e.to_string().contains("cursor position") {
                return Err(e.into());
            }
        }

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        let quit = match event::read()? {
            Event::Key(key) => {
                if app_state.slash_mode {
                    handle_key_slash(key.code, &mut app_state, shadow_engine, &mut input_buf, reflect_tx.clone()).await?
                } else if app_state.history_mode {
                    handle_key_history(key.code, &mut app_state, shadow_engine)?
                } else {
                    handle_key_normal(
                        key.code, &mut app_state, shadow_engine,
                        &mut input_buf,
                        tx.clone(), done_tx.clone(),
                    ).await?
                }
            }
            Event::Mouse(mouse) => { handle_mouse(mouse, &mut app_state); false }
            Event::Resize(..) => { app_state.scroll_offset = 0; false }
            _ => false,
        };

        if quit { 

            break; 
        }
    }

    let _ = shadow_engine.end_session();
    app_state.assistant_state = AssistantState::Idle;
    Ok(())
}


async fn process_channels(
    rx: &mut mpsc::UnboundedReceiver<String>,
    done_rx: &mut mpsc::UnboundedReceiver<()>,
    title_rx: &mut mpsc::UnboundedReceiver<String>,
    app_state: &mut TuiAppState,
    engine: &mut ShadowEngine,
    title_tx: mpsc::UnboundedSender<String>,
    reflect_rx: &mut mpsc::UnboundedReceiver<ShadowMind>,

) -> color_eyre::Result<()> {
    while let Ok(chunk) = rx.try_recv() {
        let chunk = chunk.replace("\\n", "\n");
        match engine.messages.last_mut() {
            Some(Message { kind: MessageKind::AssistantText { text }, .. }) => {
                text.push_str(&chunk);
            }
            _ => engine.messages.push(Message::agent(chunk)),
        }
        if app_state.auto_scroll {
            app_state.scroll_offset = 0;
        }
    }

    match done_rx.try_recv() {
        Ok(_) => {
            if let Some(Message { kind: MessageKind::AssistantText { text }, .. }) = engine.messages.last() {
                engine.on_stream_complete(&text.clone(), title_tx).await?;
            }
            app_state.stream_start = None;
        }
        Err(TryRecvError::Disconnected) => {
            eprintln!("stream task disconnected unexpectedly");
            app_state.stream_start = None; // reset even on disconnect
        }
        Err(TryRecvError::Empty) => {}
    }

    match title_rx.try_recv() {
        Ok(title) => {
            engine.session_name = title.clone();
            engine.db.update_session_title(engine.session_id, &title)?;
        }
        Err(TryRecvError::Disconnected) => {
            eprintln!("title generation task disconnected unexpectedly");
        }
        Err(TryRecvError::Empty) => {}
    }
    
    match reflect_rx.try_recv() {
        Ok(new_mind) => {
            engine.mind = new_mind;
            app_state.background_op_start = None;
        }
        Err(TryRecvError::Disconnected) => {
            // task died without sending — reset state
            app_state.background_op_start = None;
            eprintln!("reflect task disconnected unexpectedly");
        }
        Err(TryRecvError::Empty) => {} // still waiting, do nothing
    }

    Ok(())
}


fn update_tick(app_state: &mut TuiAppState) {
    const TICK_RATE: Duration = Duration::from_millis(100);
    if app_state.last_tick.elapsed() >= TICK_RATE {
        app_state.tick = app_state.tick.wrapping_add(1);
        app_state.last_tick = Instant::now();
    }
}


fn update_assistant_state(app_state: &mut TuiAppState) {
    app_state.assistant_state = match (app_state.stream_start, app_state.background_op_start) {
        (_, Some(start)) => AssistantState::Reflecting { secs: start.elapsed().as_secs() },
        (Some(start), None) => AssistantState::Thinking { secs: start.elapsed().as_secs() },
        (None, None) => AssistantState::Idle,
    };
}

async fn handle_key_slash(
    key: KeyCode,
    app_state: &mut TuiAppState,
    engine: &mut ShadowEngine,
    input_buf: &mut String,
    reflect_tx: mpsc::UnboundedSender<ShadowMind>
) -> color_eyre::Result<bool> {
    match key {
        KeyCode::Esc => {
            app_state.slash_mode = false;
            app_state.slash_input = String::new();
            input_buf.clear();
        }
        KeyCode::Enter => {
            let input = app_state.slash_input.trim_start_matches('/').to_lowercase();
            let matching: Vec<&SlashCommand> = SLASH_COMMANDS
                .iter()
                .filter(|cmd| cmd.name.trim_start_matches('/').starts_with(&input))
                .collect();
            let cmd = matching.get(app_state.slash_cursor).map(|c| c.name);

            app_state.slash_mode = false;
            app_state.slash_input = String::new();
            app_state.slash_cursor = 0;
            input_buf.clear();

            match SlashAction::parse(cmd.unwrap_or("")) {
                SlashAction::New => {
                    if engine.messages.len() > 1 {
                        engine.start_new_session();
                        app_state.auto_scroll = true;
                        app_state.scroll_offset = 0;
                    }
                }
                SlashAction::Delete => {
                    engine.delete_current_session()?;
                    engine.messages = engine.messages.clone();
                }
                SlashAction::History => {
                    if let Ok(sessions) = engine.list_sessions(30) {
                        app_state.history_sessions = sessions;
                        app_state.history_mode = true;
                        app_state.history_cursor = 0;
                    }
                }
                SlashAction::Ingest => {
                    match engine.ingest_icloud_logs() {
                        Ok(logs) => {
                            let mut tool = ToolCall::new("Ingest", "iCloud logs");
                            tool.finish(vec![format!("{} new logs ingested", logs.len())]);
                            tool.payload = Some(ToolPayload::Logs(logs.clone()));
                            tool.finish(logs.iter().map(|l| format!("{} — {}", format_timestamp(&l.time_stamp), l.content)).collect());
                            engine.messages.push(Message::tool(tool));
                        }
                        Err(e) => eprintln!("ingest error: {}", e),
                    }
                }
                
                SlashAction::Reflect => {
                    let (current_mind, logs_json) = gather_reflect_input(&engine.db)?;
                    let ollama = Arc::clone(&engine.ollama);
                    let tx = reflect_tx.clone();
                    app_state.stream_start = Some(Instant::now());
                    tokio::spawn(async move {
                        match reflect_with_input(&ollama, current_mind, logs_json).await {
                            Ok(new_mind) => { let _ = tx.send(new_mind); }
                            Err(e) => { eprintln!("reflect error: {}", e); }
                        }
                    });
                }
                SlashAction::Exit => {
                    if matches!(engine.assistant_state, AssistantState::Idle) {
                        return Ok(true);
                    }
                }
                SlashAction::Unknown(_) => {}
            }
        }
        KeyCode::Backspace => {
            input_buf.pop();
            if input_buf.is_empty() {
                app_state.slash_mode = false;
                app_state.slash_input = String::new();
            } else {
                app_state.slash_input = input_buf.clone();
            }
        }
        KeyCode::Up => {
            app_state.slash_cursor = app_state.slash_cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            let max = SLASH_COMMANDS.len().saturating_sub(1);
            app_state.slash_cursor = (app_state.slash_cursor + 1).min(max);
        }
        KeyCode::Char(c) => {
            input_buf.push(c);
            app_state.slash_input = input_buf.clone();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_key_history(
    key: KeyCode,
    app_state: &mut TuiAppState,
    engine: &mut ShadowEngine,
) -> color_eyre::Result<bool> {
    match key {
        KeyCode::Esc => {
            app_state.history_mode = false;
            app_state.history_sessions = vec![];
        }
        KeyCode::Enter => {
            let selected = &app_state.history_sessions[app_state.history_cursor];
            let selected_id = selected.id;
            let selected_model = selected.model.clone().unwrap_or_else(|| app_state.model.clone());
            let selected_title = selected.title.clone();

            match engine.load_session(selected_id) {
                Ok(messages) => {
                    engine.messages.clear();
                    engine.messages.push(Message::logo());
                    for msg in messages {
                        match msg.role.as_str() {
                            "user" => engine.messages.push(Message::user(msg.content)),
                            "assistant" => engine.messages.push(Message::agent(msg.content)),
                            _ => {}
                        }
                    }
                    engine.session_id = selected_id;
                    engine.session_name = selected_title;
                    engine.model = selected_model;
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
        }
        KeyCode::Up => {
            app_state.history_cursor = app_state.history_cursor.saturating_sub(1);
        }
        KeyCode::Down => {
            let max = app_state.history_sessions.len().saturating_sub(1);
            app_state.history_cursor = (app_state.history_cursor + 1).min(max);
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_key_normal(
    key: KeyCode,
    app_state: &mut TuiAppState,
    engine: &mut ShadowEngine,
    input_buf: &mut String,
    tx: mpsc::UnboundedSender<String>,
    done_tx: mpsc::UnboundedSender<()>,
) -> color_eyre::Result<bool> {
    match key {
        KeyCode::Enter => {
            let prompt = input_buf.trim().to_string();
            if prompt.is_empty() {
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

fn handle_mouse(mouse: MouseEvent, app_state: &mut TuiAppState) {
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

