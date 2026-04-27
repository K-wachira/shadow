use crate::tui::TuiAppState;
use crate::tui::tui_models::ActiveOperation;
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
    app_state.assistant_state = match &app_state.active_op {
        ActiveOperation::Idle => AssistantState::Idle,
        ActiveOperation::Streaming(start) => AssistantState::Thinking {
            secs: start.elapsed().as_secs(),
        },
        ActiveOperation::Reflecting(start) => AssistantState::Reflecting {
            secs: start.elapsed().as_secs(),
        },
        ActiveOperation::Ingesting(start) => AssistantState::Ingesting {
            secs: start.elapsed().as_secs(),
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_input_state_syncs_from_memory_edit_buffer() {
        let mut state = TuiAppState::default();
        state.memory_edit_mode = true;
        state.memory_edit_buffer = "edit content".to_string();
        sync_input_state(&mut state, "old input");
        assert_eq!(state.input, "edit content");
        assert_eq!(state.cursor_pos, 12);
    }

    #[test]
    fn sync_input_state_syncs_from_input_buf_when_not_in_edit_mode() {
        let mut state = TuiAppState::default();
        state.memory_edit_mode = false;
        state.memory_edit_buffer = "old edit".to_string();
        sync_input_state(&mut state, "new input");
        assert_eq!(state.input, "new input");
        assert_eq!(state.cursor_pos, 9);
    }

    #[test]
    fn sync_input_state_handles_multibyte_characters() {
        let mut state = TuiAppState::default();
        state.memory_edit_mode = true;
        state.memory_edit_buffer = "你好世界".to_string();
        sync_input_state(&mut state, "");
        assert_eq!(state.cursor_pos, 4);
    }

    #[test]
    fn sync_input_state_empty_input_has_zero_cursor() {
        let mut state = TuiAppState::default();
        state.memory_edit_mode = true;
        state.memory_edit_buffer = "".to_string();
        sync_input_state(&mut state, "");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn update_tick_increments_after_rate() {
        let mut state = TuiAppState::default();
        state.last_tick = Instant::now() - Duration::from_secs(1);
        update_tick(&mut state);
        assert_eq!(state.tick, 1);
    }

    #[test]
    fn update_tick_does_not_increment_before_rate() {
        let mut state = TuiAppState::default();
        state.last_tick = Instant::now();
        update_tick(&mut state);
        assert_eq!(state.tick, 0);
    }

    #[test]
    fn update_tick_resets_last_tick() {
        let mut state = TuiAppState::default();
        state.last_tick = Instant::now() - Duration::from_secs(1);
        let before = state.tick;
        update_tick(&mut state);
        assert_eq!(state.tick, before + 1);
        assert!(state.last_tick.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn update_assistant_state_idle_when_no_active_op() {
        let mut state = TuiAppState::default();
        state.active_op = ActiveOperation::Idle;
        update_assistant_state(&mut state);
        assert!(matches!(state.assistant_state, AssistantState::Idle));
    }

    #[test]
    fn update_assistant_state_streaming() {
        let mut state = TuiAppState::default();
        state.active_op = ActiveOperation::Streaming(Instant::now());
        update_assistant_state(&mut state);
        assert!(matches!(
            state.assistant_state,
            AssistantState::Thinking { .. }
        ));
    }

    #[test]
    fn update_assistant_state_reflecting() {
        let mut state = TuiAppState::default();
        state.active_op = ActiveOperation::Reflecting(Instant::now());
        update_assistant_state(&mut state);
        assert!(matches!(
            state.assistant_state,
            AssistantState::Reflecting { .. }
        ));
    }

    #[test]
    fn update_assistant_state_ingesting() {
        let mut state = TuiAppState::default();
        state.active_op = ActiveOperation::Ingesting(Instant::now());
        update_assistant_state(&mut state);
        assert!(matches!(
            state.assistant_state,
            AssistantState::Ingesting { .. }
        ));
    }

    #[test]
    fn update_assistant_state_streaming_has_elapsed_secs() {
        let start = Instant::now() - Duration::from_secs(5);
        let mut state = TuiAppState::default();
        state.active_op = ActiveOperation::Streaming(start);
        update_assistant_state(&mut state);
        if let AssistantState::Thinking { secs } = state.assistant_state {
            assert!(secs >= 5);
        } else {
            panic!("expected Thinking state");
        }
    }
}
