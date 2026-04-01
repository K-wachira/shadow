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
use crate::tui::MemoryTreeWidget;
use crate::tui::tree_render_height;
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
use ratatui::widgets::StatefulWidget;

const TREE_MAX_HEIGHT: u16 = 28;

pub fn render_chat(
    f: &mut Frame,
    area: Rect,
    tui_state: &TuiAppState,
    shadow_engine: &mut ShadowEngine,
) {
    if tui_state.history_mode {
        render_session_list(f, area, tui_state, shadow_engine);
        return;
    }

    // ── Build a flat list of segments ────────────────────────────────────────
    // Each segment is either a block of Lines (text) or a MemoryTree (widget).
    // We need total virtual height to compute scroll, same as before.

    enum Segment {
        Lines(Vec<Line<'static>>),
        Tree { msg_idx: usize, height: u16 },
    }

    let mut segments: Vec<Segment> = vec![];

    for msg in &shadow_engine.messages {
        let mut lines = message_to_lines(msg, tui_state.tick);
        all_lines.append(&mut lines);
        // Blank line after each top-level message for breathing room
        if msg.indent == 0 {
            all_lines.push(Line::from(""));
        }
    }

    // ── Compute total virtual height ─────────────────────────────────────────

    let total_virtual: usize = segments.iter().map(|s| match s {
        Segment::Lines(lines) => lines.len(),
        Segment::Tree { height, .. } => *height as usize,
    }).sum();

    let viewport_height = area.height as usize;
    let max_scroll = total_virtual.saturating_sub(viewport_height);

    let scroll_top = if tui_state.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(tui_state.scroll_offset.min(max_scroll))
    };

    // ── Walk segments, skip scrolled-past content, render visible ────────────

    let mut virtual_y: usize = 0; // position in virtual space
    let mut screen_y: u16 = area.top(); // position on screen

    for segment in &mut segments {
        if screen_y >= area.bottom() {
            break;
        }

        match segment {
            Segment::Lines(lines) => {
                let seg_height = lines.len();
                let seg_end = virtual_y + seg_height;

                if seg_end <= scroll_top {
                    // entirely scrolled past, skip
                    virtual_y = seg_end;
                    continue;
                }

                // How many lines of this segment are scrolled off the top
                let skip = if virtual_y < scroll_top { scroll_top - virtual_y } else { 0 };
                let visible_lines: Vec<Line> = lines.iter().skip(skip).cloned().collect();
                let visible_count = visible_lines.len().min((area.bottom() - screen_y) as usize);
                let visible_lines: Vec<Line> = visible_lines.into_iter().take(visible_count).collect();

                let seg_rect = Rect::new(
                    area.left(),
                    screen_y,
                    area.width,
                    visible_count as u16,
                );

                f.render_widget(
                    Paragraph::new(visible_lines).wrap(Wrap { trim: false }),
                    seg_rect,
                );

                screen_y += visible_count as u16;
                virtual_y = seg_end;
            }

            Segment::Tree { msg_idx, height } => {
                let seg_height = *height as usize;
                let seg_end = virtual_y + seg_height;
            
                if seg_end <= scroll_top {
                    virtual_y = seg_end;
                    continue;
                }
            
                let remaining = (area.bottom() - screen_y) as u16;
                let render_height = (*height).min(remaining);
            
                if render_height == 0 {
                    virtual_y = seg_end;
                    continue;
                }
            
                let tree_rect = Rect::new(area.left(), screen_y, area.width, render_height);
                let focused = tui_state.memory_focus == Some(*msg_idx);
            
                if let Some(Message { kind: MessageKind::MemoryTree(tree), .. }) =
                    shadow_engine.messages.get_mut(*msg_idx)
                {
                    MemoryTreeWidget { focused, max_height: render_height }
                        .render(tree_rect, f.buffer_mut(), tree);
                }
            
                screen_y += render_height;
                virtual_y = seg_end;
            }
        }
    }
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
        // MemoryTree is handled upstream in render_chat, never reaches here
        MessageKind::MemoryTree(_) => vec![],
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
                    Span::styled(format!("{:<42} {}", title, format_timestamp(&session.created_at_ms.to_string())), dim()),
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
                    " sessions  (↑↓ or j/k to navigate · Enter to load · Esc to cancel) ",
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