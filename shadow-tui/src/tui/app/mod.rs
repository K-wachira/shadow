mod channels;
mod handlers;
mod state;
use crate::tui::TuiAppState;
use crate::tui::flush_chat_transcript;
use crate::tui::persist_chat_scrollback;
use crate::tui::render;
use crossterm::event::Event;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use shadow_continuity::mind::ShadowMind;
use shadow_core::locus::Locus;
use shadow_core::model::AssistantState;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;

use self::channels::process_channels;
use self::handlers::handle_key_history;
use self::handlers::handle_key_logs;
use self::handlers::handle_key_normal;
use self::handlers::handle_key_slash;
use self::handlers::handle_mouse;
use self::handlers::handle_pending_confirm_key;
use self::state::sync_input_state;
use self::state::update_assistant_state;
use self::state::update_tick;
use shadow_services::ingest::get_files;

pub async fn run(mut terminal: DefaultTerminal, locus: &mut Locus) -> color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (done_tx, mut done_streaming_rx) = mpsc::unbounded_channel::<()>();
    let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
    let (reflect_tx, mut reflect_rx) = mpsc::unbounded_channel::<ShadowMind>();
    let (ingest_tx, mut ingest_rx) = mpsc::unbounded_channel::<()>();
    let mut app_state = TuiAppState::default();
    let mut input_buf = String::new();

    // spawn watcher once
    start_ingest_watcher(locus.config.ingest.source_path.clone(), ingest_tx);

    loop {
        process_channels(
            &mut rx,
            &mut done_streaming_rx,
            &mut title_rx,
            &mut app_state,
            locus,
            title_tx.clone(),
            &mut reflect_rx,
            &mut ingest_rx,
        )
        .await?;

        update_tick(&mut app_state);
        update_assistant_state(&mut app_state);
        sync_input_state(&mut app_state, &input_buf);

        // Defer scrollback persistence while the terminal is actively resizing
        // — each intermediate width would otherwise re-emit the whole
        // transcript. Once resizes settle, reset once and re-emit at the new
        // width.
        if !app_state.resize_settling() {
            if app_state.resize_pending.take().is_some() {
                app_state.reset_persisted_chat();
            }
            persist_chat_scrollback(&mut terminal, &mut app_state, locus)?;
        }

        if let Err(e) = terminal.draw(|f| render(f, &app_state, locus)) {
            if !e.to_string().contains("cursor position") {
                return Err(e.into());
            }
        }

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }

        let quit = match event::read()? {
            Event::Key(key) => {
                if app_state.pending_confirm.is_some() {
                    handle_pending_confirm_key(
                        key.code,
                        &mut app_state,
                        locus,
                        &mut input_buf,
                        reflect_tx.clone(),
                    )
                    .await?
                } else if app_state.slash_mode {
                    handle_key_slash(
                        key.code,
                        &mut app_state,
                        locus,
                        &mut input_buf,
                        reflect_tx.clone(),
                    )
                    .await?
                } else if app_state.history_mode {
                    handle_key_history(key.code, &mut app_state, locus)?
                } else if app_state.logs_mode {
                    handle_key_logs(key.code, &mut app_state)?
                } else {
                    handle_key_normal(
                        key,
                        &mut app_state,
                        locus,
                        &mut input_buf,
                        tx.clone(),
                        done_tx.clone(),
                    )
                    .await?
                }
            }
            Event::Mouse(mouse) => {
                handle_mouse(mouse, &mut app_state, locus)?;
                false
            }
            Event::Resize(..) => {
                terminal.autoresize()?;
                app_state.resize_pending = Some(Instant::now());
                app_state.scroll_offset = 0;
                false
            }
            _ => false,
        };

        if quit {
            break;
        }
    }

    flush_chat_transcript(&mut terminal, &mut app_state, locus)?;
    let _ = locus.end_session();
    app_state.assistant_state = AssistantState::Idle;
    Ok(())
}

pub fn start_ingest_watcher(source_path: PathBuf, tx: mpsc::UnboundedSender<()>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        let mut last_seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        loop {
            interval.tick().await;
            if let Ok(files) = get_files(&source_path) {
                let current: std::collections::HashSet<String> = files
                    .into_iter()
                    .filter(|f| f.contains(".json") && !f.starts_with('.'))
                    .collect();

                if current != last_seen {
                    last_seen = current;
                    let _ = tx.send(());
                }
            }
        }
    });
}
