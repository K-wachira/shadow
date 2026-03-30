use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use shadow_core::model::ToolCall;
use shadow_core::model::ToolState;
use shadow_core::engine::ShadowEngine;
use shadow_core::utils::format_timestamp;
use shadow_core::utils::truncate;
use crate::tui::TuiAppState;
use crate::tui::bright_bold;
use crate::tui::default;
use crate::tui::dim;
use crate::tui::logo_lines;
use crate::tui::markdown_to_lines;
use crate::tui::muted;
use crate::tui::very_dim;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

pub fn render_chat(f: &mut Frame, area: Rect, tui_state: &TuiAppState, shadow_engine: &mut ShadowEngine) {
    if tui_state.history_mode {
        render_session_list(f, area, tui_state, shadow_engine);
        return;
    }

    let mut all_lines: Vec<Line> = vec![];

    for msg in &shadow_engine.messages {
        let mut lines = message_to_lines(msg, tui_state.tick);
        all_lines.append(&mut lines);
        // Blank line after each top-level message for breathing room
        if msg.indent == 0 {
            all_lines.push(Line::from(""));
        }
    }

    let height = area.height as usize;

    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(height);
    
    let scroll_row = if tui_state.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(tui_state.scroll_offset.min(max_scroll))
    };  
    f.render_widget(
        Paragraph::new(all_lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll_row as u16, 0)),
        area,
    );
}

fn message_to_lines(msg: &Message, tick: u64) -> Vec<Line<'static>> {
    let pad = "  ".repeat(msg.indent as usize);

    match &msg.kind {
        MessageKind::Logo { text } => logo_lines(text),

        MessageKind::UserInput { text } => vec![Line::from(vec![
            Span::raw(format!("{}❯  ", pad)),
            Span::styled(text.clone(), default()), // was bright()
        ])],

        MessageKind::AssistantThought { text } => vec![Line::from(vec![
            Span::styled(format!("{}+  ", pad), Style::default().fg(Color::Blue)),
            Span::styled(text.clone(), default()),
        ])],

        MessageKind::AssistantText { text } => {
            let mut lines = markdown_to_lines(&text);
            // prepend the ">" indicator on the first line
            if let Some(first) = lines.first_mut() {
                first
                    .spans
                    .insert(0, Span::styled(format!("{}❯  ", pad), default() ));
            }
            lines
        }
        MessageKind::Tool(tool) => tool_to_lines(tool, &pad, tick),
    }
}

fn render_session_list(f: &mut Frame, area: Rect, tui_state: &TuiAppState, shadow_engine: &mut ShadowEngine ) {
   let history_sessions = shadow_engine.list_sessions(30).unwrap(); //TODO Handle error case
    if history_sessions.is_empty() {
        let line = Line::from(Span::styled("  no sessions found", dim()));
        f.render_widget(Paragraph::new(line), area);
        return;

    }

    let items: Vec<Line> = history_sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let title = if session.title.len() > 40 {
                format!("{}…", &session.title[..40])
            } else {
                session.title.clone()
            };

            if i == tui_state.history_cursor {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<42} {}", title, format_timestamp(&session.created_at_ms.to_string())),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{:<42} {}", title, &session.created_at_ms.to_string()), dim()),
                ])
            }
        })
        .collect();
    

    f.render_widget(
        Paragraph::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    " sessions  (↑↓ · Enter to load · Esc to cancel) ",
                    dim(),
                )),
        ),
        area,
    );
}

fn tool_to_lines(tool: &ToolCall, pad: &str, tick: u64) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![];

    match &tool.state {
        ToolState::Running => {
            // Spinning: "⠋ Shell  (ctrl+f to focus)"
            let sp = match tick % 4 {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                _ => "⠸",
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}{}  ", pad, sp),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(tool.name.to_owned(), bright_bold()),
                Span::styled("  (ctrl+f to focus)".to_string(), dim()),
            ]));
            // Show last live stdout line if streaming
            if let Some(last) = tool.output_lines.last() {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}└  ", pad), dim()),
                    Span::styled(
                        last.to_owned(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::DIM),
                    ),
                ]));
            }
        }

        ToolState::Collapsed => {
            // "● Shell(python3 -c "…") (3 lines)  (Ctrl+O to expand)"
            let n = tool.output_lines.len();
            let word = if n == 1 { "line" } else { "lines" };
            let preview = if tool.args_preview.is_empty() {
                String::new()
            } else {
                format!("({})", truncate(&tool.args_preview, 48))
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{}●  ", pad), Style::default().fg(Color::Green)),
                Span::styled(format!("{}{}", tool.name, preview), muted()),
                Span::styled(format!("  ({} {})", n, word), dim()),
                Span::styled("  (Ctrl+O to expand)".to_string(), very_dim()),
            ]));
        }

        ToolState::Expanded => {
            // "● Shell"
            // "└  $ mkdir -p /path"
            lines.push(Line::from(vec![
                Span::styled(format!("{}●  ", pad), Style::default().fg(Color::Green)),
                Span::styled(tool.name.clone(), bright_bold()),
            ]));
            for output_line in &tool.output_lines {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}└  ", pad), dim()),
                    Span::styled(truncate(&output_line.to_owned(), 80), muted()),
                ]));
            }
        }
    }

    // Children (Worker pattern):
    // └  ✓ Shell(mkdir -p ...)
    for child in &tool.children {
        let check = if child.completed { "✓ " } else { "" };
        let preview = if child.args_preview.is_empty() {
            String::new()
        } else {
            format!("({})", truncate(&child.args_preview, 40))
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}└  ", pad), dim()),
            Span::styled(format!("{}{}{}", check, child.name, preview), dim()),
        ]));
    }
    lines
}