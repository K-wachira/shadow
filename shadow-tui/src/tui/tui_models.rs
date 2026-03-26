use shadow_core::model::AssistantState;
use shadow_core::db::Sessions;

#[derive(Debug)]
pub struct TuiAppState {
    pub input: String,          // what the user is typing
    pub model: String, // "gemma3:12b"
    pub yolo_mode: bool,
    pub assistant_state: AssistantState,
    // pub context_logs: Vec<String>, // logs shown in right panel

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
}

impl Default for TuiAppState {
    fn default() -> Self {
        let state = Self {
            input: String::new(),
            model: "llama3.2".to_string(),
            assistant_state: AssistantState::Idle,

            cursor_pos: 0,
            scroll_offset: 0,
            auto_scroll: true,
            tick: 0,
            yolo_mode: false,
            // context_logs: vec![],

            slash_mode: false,   // typing a slash command
            history_mode: false, // navigating session list
            slash_input: String::new(),   // what's been typed after "/"
            history_sessions: vec![],
            history_cursor: 0,
        };

        // Logo is always the first message — it scrolls away as history grows.
        // No conditional rendering needed.
        state
    }
}

pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand { name: "/history", description: "Show past sessions" },
];

