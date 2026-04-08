use shadow_services::models::EntryLog;
use crate::json_tree::JsonTree;

#[derive(Debug, Clone)]
pub enum MessageKind {
    Logo { text: String },

    UserInput { text: String },

    AssistantThought { text: String },

    AssistantText { text: String },

    Tool(ToolCall),

    MemoryTree(JsonTree),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub kind: MessageKind,
    /// Indent depth: 0 = root, 1 = inside Worker subagent, etc.
    pub indent: u8,
}

impl Message {
    pub fn logo(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Logo { text: text.into() },
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

    pub fn memory_tree(tree: JsonTree) -> Self {
        Self {
            kind: MessageKind::MemoryTree(tree),
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

    Reflecting {
        secs: u64,
    },

    Ingesting {
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
            AssistantState::Reflecting { secs } => {
                Some(format!("Reflecting…  (esc to cancel, {}s)", secs))
            }
            AssistantState::Ingesting { secs } => {
                Some(format!("Ingesting…  (esc to cancel, {}s)", secs))
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
    pub payload: Option<ToolPayload>,
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
            payload: None,
        }
    }

    pub fn finish(&mut self, output: Vec<String>) {
        self.output_lines = output;
        self.completed = true;
        self.state = ToolState::Expanded;
    }

    pub fn toggle_expand(&mut self) {
        self.state = match self.state {
            ToolState::Collapsed => ToolState::Expanded,
            ToolState::Expanded => ToolState::Collapsed,
            ToolState::Running => ToolState::Running,
        };
    }
}

#[derive(Debug, Clone)]
pub enum ToolPayload {
    Logs(Vec<EntryLog>),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Message constructors ──────────────────────────────────────────────────

    #[test]
    fn message_user_sets_kind_and_zero_indent() {
        let msg = Message::user("hello");
        assert!(matches!(msg.kind, MessageKind::UserInput { text } if text == "hello"));
        assert_eq!(msg.indent, 0);
    }

    #[test]
    fn message_agent_sets_assistant_text_kind() {
        let msg = Message::agent("reply");
        assert!(matches!(msg.kind, MessageKind::AssistantText { text } if text == "reply"));
        assert_eq!(msg.indent, 0);
    }

    #[test]
    fn message_thought_sets_thought_kind() {
        let msg = Message::thought("pondering");
        assert!(matches!(msg.kind, MessageKind::AssistantThought { text } if text == "pondering"));
    }

    #[test]
    fn message_logo_sets_logo_kind() {
        let msg = Message::logo("Model");
        assert!(matches!(msg.kind, MessageKind::Logo{ text } if text == "Model"));
        assert_eq!(msg.indent, 0);
    }

    #[test]
    fn message_tool_wraps_tool_call() {
        let call = ToolCall::new("Shell", "ls -la");
        let msg = Message::tool(call);
        assert!(matches!(msg.kind, MessageKind::Tool(_)));
    }

    // ── AssistantState ────────────────────────────────────────────────────────

    #[test]
    fn idle_is_not_active() {
        assert!(!AssistantState::Idle.is_active());
    }

    #[test]
    fn thinking_is_active() {
        assert!(AssistantState::Thinking { secs: 0 }.is_active());
    }

    #[test]
    fn choosing_is_active() {
        assert!(AssistantState::Choosing { secs: 5 }.is_active());
    }

    #[test]
    fn preparing_is_active() {
        assert!(AssistantState::Preparing { secs: 2 }.is_active());
    }

    #[test]
    fn refining_is_active() {
        assert!(AssistantState::Refining { secs: 1 }.is_active());
    }

    #[test]
    fn idle_status_text_is_none() {
        assert!(AssistantState::Idle.status_text().is_none());
    }

    #[test]
    fn thinking_status_text_includes_seconds() {
        let text = AssistantState::Thinking { secs: 7 }.status_text().unwrap();
        assert!(text.contains("7s"));
        assert!(text.contains("Thinking"));
    }

    #[test]
    fn choosing_status_text_includes_seconds() {
        let text = AssistantState::Choosing { secs: 3 }.status_text().unwrap();
        assert!(text.contains("3s"));
        assert!(text.contains("Choosing"));
    }

    #[test]
    fn idle_spinner_returns_space() {
        assert_eq!(AssistantState::Idle.spinner(0), " ");
    }

    #[test]
    fn active_spinner_cycles_with_tick() {
        let state = AssistantState::Thinking { secs: 0 };
        let frames: Vec<&str> = (0..4).map(|t| state.spinner(t)).collect();
        // All four ticks should produce different spinner chars
        assert_eq!(frames, vec!["⠋", "⠙", "⠹", "⠸"]);
    }

    #[test]
    fn spinner_wraps_at_4() {
        let state = AssistantState::Thinking { secs: 0 };
        assert_eq!(state.spinner(0), state.spinner(4));
        assert_eq!(state.spinner(1), state.spinner(5));
    }

    #[test]
    fn idle_input_prefix_is_greater_than() {
        assert_eq!(AssistantState::Idle.input_prefix(), ">");
    }

    #[test]
    fn active_input_prefix_is_asterisk() {
        assert_eq!(AssistantState::Thinking { secs: 0 }.input_prefix(), "*");
    }

    // ── ToolCall ──────────────────────────────────────────────────────────────

    #[test]
    fn new_tool_call_starts_running_not_completed() {
        let tool = ToolCall::new("Shell", "echo hi");
        assert_eq!(tool.name, "Shell");
        assert_eq!(tool.args_preview, "echo hi");
        assert_eq!(tool.state, ToolState::Running);
        assert!(!tool.completed);
        assert!(tool.output_lines.is_empty());
        assert!(tool.children.is_empty());
    }

    #[test]
    fn finish_sets_collapsed_and_completed() {
        let mut tool = ToolCall::new("Shell", "ls");
        tool.finish(vec!["file1.txt".into(), "file2.txt".into()]);
        assert!(tool.completed);
        assert_eq!(tool.state, ToolState::Collapsed);
        assert_eq!(tool.output_lines.len(), 2);
        assert_eq!(tool.output_lines[0], "file1.txt");
    }

    #[test]
    fn toggle_expand_collapsed_becomes_expanded() {
        let mut tool = ToolCall::new("Shell", "");
        tool.finish(vec![]);
        assert_eq!(tool.state, ToolState::Collapsed);
        tool.toggle_expand();
        assert_eq!(tool.state, ToolState::Expanded);
    }

    #[test]
    fn toggle_expand_expanded_becomes_collapsed() {
        let mut tool = ToolCall::new("Shell", "");
        tool.finish(vec![]);
        tool.toggle_expand(); // Collapsed → Expanded
        tool.toggle_expand(); // Expanded → Collapsed
        assert_eq!(tool.state, ToolState::Collapsed);
    }

    #[test]
    fn toggle_expand_running_stays_running() {
        let mut tool = ToolCall::new("Shell", "");
        assert_eq!(tool.state, ToolState::Running);
        tool.toggle_expand();
        assert_eq!(tool.state, ToolState::Running);
    }
}
