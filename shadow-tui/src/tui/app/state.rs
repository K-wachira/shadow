use crate::tui::TuiAppState;
use shadow_core::model::AssistantState;
use std::time::Duration;
use std::time::Instant;

pub fn sync_input_state(app_state: &mut TuiAppState, input_buf: &str) {
    if app_state.memory_edit_mode {
        app_state.input = app_state.memory_edit_buffer.clone();
        app_state.cursor_pos = app_state.memory_edit_buffer.chars().count();
    } else {
        app_state.input = input_buf.to_string();
        app_state.cursor_pos = input_buf.chars().count();
    }
}

pub fn update_tick(app_state: &mut TuiAppState) {
    const TICK_RATE: Duration = Duration::from_millis(100);
    if app_state.last_tick.elapsed() >= TICK_RATE {
        app_state.tick = app_state.tick.wrapping_add(1);
        app_state.last_tick = Instant::now();
    }
}

pub fn update_assistant_state(app_state: &mut TuiAppState) {
    app_state.assistant_state = match (app_state.stream_start, app_state.background_op_start) {
        (_, Some(start)) => AssistantState::Reflecting {
            secs: start.elapsed().as_secs(),
        },
        (Some(start), None) => AssistantState::Thinking {
            secs: start.elapsed().as_secs(),
        },
        (None, None) => AssistantState::Idle,
    };
}
