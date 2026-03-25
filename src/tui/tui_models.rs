use crate::db::Sessions;

#[derive(Debug)]
pub struct AppState {
    pub messages: Vec<Message>, // conversation history
    pub input: String,          // what the user is typing
    pub session_id: i64,

    pub session_name: Option<String>,
    pub model: String, // "gemma3:12b"
    pub yolo_mode: bool,
    pub context_logs: Vec<String>, // logs shown in right panel
    pub assistant_state: AssistantState,

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

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            messages: vec![],
            input: String::new(),
            session_id: 0,
            session_name: None,
            model: "llama3.2".to_string(),
            assistant_state: AssistantState::Idle,

            cursor_pos: 0,
            scroll_offset: 0,
            auto_scroll: true,
            tick: 0,
            yolo_mode: false,
            context_logs: vec![],

            slash_mode: false,   // typing a slash command
            history_mode: false, // navigating session list
            slash_input: String::new(),   // what's been typed after "/"
            history_sessions: vec![],
            history_cursor: 0,
        };

        // Logo is always the first message — it scrolls away as history grows.
        // No conditional rendering needed.
        state.messages.push(Message::logo());
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

#[derive(Debug, Clone)]
pub enum MessageKind {
    Logo,

    UserInput { text: String },

    AssistantThought { text: String },

    AssistantText { text: String },

    Tool(ToolCall),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub kind: MessageKind,
    /// Indent depth: 0 = root, 1 = inside Worker subagent, etc.
    pub indent: u8,
}

impl Message {
    pub fn logo() -> Self {
        Self {
            kind: MessageKind::Logo,
            indent: 0,
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::UserInput { text: text.into() },
            indent: 0,
        }
    }

    pub fn thought(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::AssistantThought { text: text.into() },
            indent: 0,
        }
    }

    pub fn agent(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::AssistantText { text: text.into() },
            indent: 0,
        }
    }

    pub fn tool(call: ToolCall) -> Self {
        Self {
            kind: MessageKind::Tool(call),
            indent: 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum AssistantState {
    #[default]
    Idle,
    Choosing {
        secs: u64,
    },
    Thinking {
        secs: u64,
    },
    Preparing {
        secs: u64,
    },
    Refining {
        secs: u64,
    },
}

impl AssistantState {
    pub fn is_active(&self) -> bool {
        !matches!(self, AssistantState::Idle)
    }

    /// Italic line shown above input while agent is working. None = blank line.
    pub fn status_text(&self) -> Option<String> {
        match self {
            AssistantState::Idle => None,
            AssistantState::Choosing { secs } => {
                Some(format!("Choosing…  (esc to cancel, {}s)", secs))
            }
            AssistantState::Thinking { secs } => {
                Some(format!("Thinking…  (esc to cancel, {}s)", secs))
            }
            AssistantState::Preparing { secs } => {
                Some(format!("Preparing… (esc to cancel, {}s)", secs))
            }
            AssistantState::Refining { secs } => {
                Some(format!("Refining…  (esc to cancel, {}s)", secs))
            }
        }
    }

    /// Braille spinner — cycles on each app tick.
    pub fn spinner(&self, tick: u64) -> &'static str {
        if !self.is_active() {
            return " ";
        }
        match tick % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        }
    }

    /// ">" idle, "*" active — matches OB-1 exactly.
    pub fn input_prefix(&self) -> &'static str {
        if self.is_active() { "*" } else { ">" }
    }
}

// ─── Tool call ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ToolState {
    /// Agent is currently executing this tool.
    /// Renders: "⠋ Shell  (ctrl+f to focus)"
    Running,

    /// Tool finished, output hidden.
    /// Renders: "● Shell(args…) (N lines)  (Ctrl+O to expand)"
    Collapsed,

    /// Tool finished, output visible.
    /// Renders: "● Shell\n└  $ line1\n└  $ line2"
    Expanded,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    /// Short preview of args shown when collapsed/in child view
    pub args_preview: String,
    pub output_lines: Vec<String>,
    pub state: ToolState,
    pub completed: bool,
    /// Subagent children — e.g. Worker spawning Shell calls
    pub children: Vec<ToolCall>,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, args_preview: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args_preview: args_preview.into(),
            output_lines: vec![],
            state: ToolState::Running,
            completed: false,
            children: vec![],
        }
    }

    pub fn finish(&mut self, output: Vec<String>) {
        self.output_lines = output;
        self.completed = true;
        self.state = ToolState::Collapsed;
    }

    pub fn toggle_expand(&mut self) {
        self.state = match self.state {
            ToolState::Collapsed => ToolState::Expanded,
            ToolState::Expanded => ToolState::Collapsed,
            ToolState::Running => ToolState::Running,
        };
    }
}
