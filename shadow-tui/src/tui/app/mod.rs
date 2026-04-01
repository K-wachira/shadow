mod channels;
mod handlers;
mod state;

use crate::tui::render;
use crate::tui::TuiAppState;
use crossterm::event::Event;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use shadow_core::engine::ShadowEngine;
use shadow_core::mind::ShadowMind;
use shadow_core::model::AssistantState;
use std::time::Duration;
use tokio::sync::mpsc;

use self::channels::process_channels;
use self::handlers::handle_key_history;
use self::handlers::handle_key_normal;
use self::handlers::handle_key_slash;
use self::handlers::handle_mouse;
use self::state::sync_input_state;
use self::state::update_assistant_state;
use self::state::update_tick;

pub async fn run(
    mut terminal: DefaultTerminal,
    shadow_engine: &mut ShadowEngine,
) -> color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<()>();
    let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
    let (reflect_tx, mut reflect_rx) = mpsc::unbounded_channel::<ShadowMind>();
    let mut app_state = TuiAppState::default();
    let mut input_buf = String::new();

    loop {
        process_channels(
            &mut rx,
            &mut done_rx,
            &mut title_rx,
            &mut app_state,
            shadow_engine,
            title_tx.clone(),
            &mut reflect_rx,
        )
        .await?;

        update_tick(&mut app_state);
        update_assistant_state(&mut app_state);
        sync_input_state(&mut app_state, &input_buf);

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
                    handle_key_slash(
                        key.code,
                        &mut app_state,
                        shadow_engine,
                        &mut input_buf,
                        reflect_tx.clone(),
                    )
                    .await?
                } else if app_state.history_mode {
                    handle_key_history(key.code, &mut app_state, shadow_engine)?
                } else {
                    handle_key_normal(
                        key.code,
                        &mut app_state,
                        shadow_engine,
                        &mut input_buf,
                        tx.clone(),
                        done_tx.clone(),
                    )
                    .await?
                }
            }
            Event::Mouse(mouse) => {
                handle_mouse(mouse, &mut app_state);
                false
            }
            Event::Resize(..) => {
                app_state.scroll_offset = 0;
                false
            }
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
