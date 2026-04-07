use crate::tui::TuiAppState;
use shadow_core::engine::ShadowEngine;
use shadow_core::mind::ShadowMind;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use crate::tui::tui_models::ActiveOperation;

pub async fn process_channels(
    rx: &mut mpsc::UnboundedReceiver<String>, done_streaming_rx: &mut mpsc::UnboundedReceiver<()>,
    title_rx: &mut mpsc::UnboundedReceiver<String>, app_state: &mut TuiAppState,
    engine: &mut ShadowEngine, title_tx: mpsc::UnboundedSender<String>,
    reflect_rx: &mut mpsc::UnboundedReceiver<ShadowMind>,
) -> color_eyre::Result<()> {
    while let Ok(chunk) = rx.try_recv() {
        let chunk = chunk.replace("\\n", "\n");
        match engine.messages.last_mut() {
            Some(Message {
                kind: MessageKind::AssistantText { text },
                ..
            }) => {
                text.push_str(&chunk);
            }
            _ => engine.messages.push(Message::agent(chunk)),
        }
        if app_state.auto_scroll {
            app_state.scroll_offset = 0;
        }
    }

    match done_streaming_rx.try_recv() {
        Ok(_) => {
            if let Some(Message {
                kind: MessageKind::AssistantText { text },
                ..
            }) = engine.messages.last()
            {
                engine.on_stream_complete(&text.clone(), title_tx).await?;
            }
            app_state.active_op = ActiveOperation::Idle;
        }
        Err(TryRecvError::Disconnected) => {
            tracing::error!("stream task disconnected unexpectedly");
            app_state.active_op = ActiveOperation::Idle;
        }
        Err(TryRecvError::Empty) => {}
    }

    match title_rx.try_recv() {
        Ok(title) => {
            engine.session_name = title.clone();
            engine.db.update_session_title(engine.session_id, &title)?;
        }
        Err(TryRecvError::Disconnected) => {
            tracing::error!("title generation task disconnected unexpectedly");
        }
        Err(TryRecvError::Empty) => {}
    }

    match reflect_rx.try_recv() {
        Ok(new_mind) => {
            engine.mind = new_mind;
            app_state.active_op = ActiveOperation::Idle;
        }
        Err(TryRecvError::Disconnected) => {
            app_state.active_op = ActiveOperation::Idle;
            tracing::error!("reflect task disconnected unexpectedly");
        }
        Err(TryRecvError::Empty) => {}
    }

    Ok(())
}
