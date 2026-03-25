
#[derive(Debug, Clone)]
pub enum ShadowEngine {
}

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