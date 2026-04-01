use shadow_core::model::AssistantState;
use shadow_core::db::Sessions;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
pub struct TuiAppState {
    pub input: String,          // what the user is typing
    pub yolo_mode: bool,
    pub assistant_state: AssistantState,
    pub rename_mode: bool,
    pub cursor_pos: usize,
    /// Lines scrolled up from the bottom (0 = latest content visible)
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    /// Monotonic tick counter — increment on each terminal tick event (~100ms)
    pub tick: u64,

    pub slash_mode: bool,            // typing a slash command
    pub slash_input: String, // what's been typed after "/"
    pub history_mode: bool,          // navigating session list
    pub history_sessions: Vec<Sessions>,
    pub history_cursor: usize,
    pub slash_cursor: usize,
    pub last_tick: Instant,
    pub stream_start: Option<Instant>,
    pub background_op_start: Option<Instant>,
    pub memory_focus: Option<usize>,
    pub memory_edit_mode: bool,
    pub memory_edit_buffer: String,
    pub memory_edit_path: Option<Vec<usize>>,
    pub memory_source_path: Option<PathBuf>,
}

impl Default for TuiAppState {
    fn default() -> Self {
        let state = Self {
            input: String::new(),
            memory_focus: None,
            assistant_state: AssistantState::Idle,

            cursor_pos: 0,
            scroll_offset: 0,
            auto_scroll: true,
            tick: 0,
            yolo_mode: false,
            // context_logs: vec![],
            rename_mode: false,
            slash_mode: false,   // typing a slash command
            history_mode: false, // navigating session list
            slash_input: String::new(),   // what's been typed after "/"
            history_sessions: vec![],
            history_cursor: 0,
            slash_cursor: 0,
            
            last_tick: Instant::now(),
            stream_start: None,
            background_op_start: None,
            memory_edit_mode: false,
            memory_edit_buffer: String::new(),
            memory_edit_path: None,
            memory_source_path: None,
        };
        state
    }
}

pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand { name: "/new", description: "start new session" },
    SlashCommand { name: "/history", description: "list past sessions" },
    SlashCommand { name: "/refect", description: "reflect .." },
    SlashCommand { name: "/delete", description: "delete current session" },
    SlashCommand { name: "/ingest", description: "ingest new logs from icloud" },
    SlashCommand { name: "/rename", description: "rename session title" },
    SlashCommand { name: "/memory", description: "memory ..." },
    SlashCommand { name: "/exit", description: "exit Shadow" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_correct_initial_values() {
        let state = TuiAppState::default();
        assert!(state.input.is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert!(state.auto_scroll);
        assert_eq!(state.tick, 0);
        assert!(!state.slash_mode);
        assert!(!state.history_mode);
        assert!(!state.yolo_mode);
        assert!(state.slash_input.is_empty());
        assert!(state.history_sessions.is_empty());
        assert_eq!(state.history_cursor, 0);
        assert_eq!(state.slash_cursor, 0);
    }

    #[test]
    fn slash_commands_list_contains_expected_commands() {
        let names: Vec<&str> = SLASH_COMMANDS.iter().map(|c| c.name).collect();
        assert!(names.contains(&"/new"));
        assert!(names.contains(&"/delete"));
        assert!(names.contains(&"/history"));
        assert!(names.contains(&"/exit"));
    }

    #[test]
    fn slash_commands_all_have_descriptions() {
        for cmd in SLASH_COMMANDS {
            assert!(!cmd.description.is_empty(), "{} has empty description", cmd.name);
        }
    }

    #[test]
    fn slash_commands_names_start_with_slash() {
        for cmd in SLASH_COMMANDS {
            assert!(cmd.name.starts_with('/'), "{} doesn't start with /", cmd.name);
        }
    }
}
