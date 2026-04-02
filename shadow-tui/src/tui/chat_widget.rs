use crate::tui::SLASH_COMMANDS;
use crate::tui::TuiAppState;
use crate::tui::bright_bold;
use crate::tui::default;
use crate::tui::dim;
use crate::tui::error_style;
use crate::tui::logo_lines;
use crate::tui::markdown_to_lines;
use crate::tui::muted;
use crate::tui::tree_cursor_screen_row;
use crate::tui::tree_render_height;
use crate::tui::tree_to_lines;
use crate::tui::very_dim;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use shadow_core::engine::ShadowEngine;
use shadow_core::model::Message;
use shadow_core::model::MessageKind;
use shadow_core::model::ToolCall;
use shadow_core::model::ToolState;
use shadow_core::utils::format_timestamp;
use shadow_core::utils::truncate;

#[derive(Clone)]
enum Segment {
    Lines(Vec<Line<'static>>),
}

pub fn render_chat(
    f: &mut Frame, area: Rect, tui_state: &TuiAppState, shadow_engine: &mut ShadowEngine,
) {
    if tui_state.history_mode {
        render_session_list(f, area, tui_state, shadow_engine);
        return;
    }

    f.render_widget(Clear, area);

    let segments = build_segments(shadow_engine, tui_state);
    let total_rows = total_segment_rows(&segments, area.width);
    let visible_top = visible_top(total_rows, area.height as usize, tui_state);
    render_chat_slice(
        f.buffer_mut(),
        area,
        tui_state,
        shadow_engine,
        &segments,
        visible_top,
    );
}

pub fn persist_chat_scrollback(
    terminal: &mut DefaultTerminal, tui_state: &mut TuiAppState, shadow_engine: &mut ShadowEngine,
) -> std::io::Result<()> {
    sync_chat_scrollback(terminal, tui_state, shadow_engine, false)
}

pub fn flush_chat_transcript(
    terminal: &mut DefaultTerminal, tui_state: &mut TuiAppState, shadow_engine: &mut ShadowEngine,
) -> std::io::Result<()> {
    sync_chat_scrollback(terminal, tui_state, shadow_engine, true)
}

pub fn ensure_memory_cursor_visible(
    tui_state: &mut TuiAppState, shadow_engine: &ShadowEngine,
) -> color_eyre::Result<()> {
    let Some(focus_idx) = tui_state.memory_focus else {
        return Ok(());
    };

    let terminal_size = crossterm::terminal::size()?;
    let total_area = Rect::from((
        ratatui::layout::Position::ORIGIN,
        ratatui::layout::Size::new(terminal_size.0, terminal_size.1),
    ));
    let chat_area = chat_area(total_area, tui_state);
    if chat_area.height == 0 || chat_area.width == 0 {
        return Ok(());
    }

    let Some(target_row) =
        memory_cursor_row(focus_idx, shadow_engine, tui_state.tick, chat_area.width)
    else {
        return Ok(());
    };

    let segments = build_segments(shadow_engine, tui_state);
    let total_rows = total_segment_rows(&segments, chat_area.width);
    let viewport_rows = chat_area.height as usize;
    let max_scroll = total_rows.saturating_sub(viewport_rows);
    let current_top = visible_top(total_rows, viewport_rows, tui_state);
    let current_bottom = current_top.saturating_add(viewport_rows.saturating_sub(1));

    let desired_top = if target_row < current_top {
        target_row
    } else if target_row > current_bottom {
        target_row.saturating_sub(viewport_rows.saturating_sub(1))
    } else {
        return Ok(());
    }
    .min(max_scroll);

    tui_state.scroll_offset = max_scroll.saturating_sub(desired_top);
    tui_state.auto_scroll = tui_state.scroll_offset == 0;
    Ok(())
}

fn sync_chat_scrollback(
    terminal: &mut DefaultTerminal, tui_state: &mut TuiAppState, shadow_engine: &mut ShadowEngine,
    include_visible_viewport: bool,
) -> std::io::Result<()> {
    if tui_state.history_mode {
        tui_state.persisted_chat_rows = 0;
        tui_state.persisted_chat_width = 0;
        return Ok(());
    }

    if tui_state.slash_mode {
        return Ok(());
    }

    let total_area = Rect::from((ratatui::layout::Position::ORIGIN, terminal.size()?));
    let chat_area = chat_area(total_area, tui_state);
    if chat_area.height == 0 || chat_area.width == 0 {
        tui_state.persisted_chat_rows = 0;
        tui_state.persisted_chat_width = chat_area.width;
        return Ok(());
    }

    if tui_state.persisted_chat_width != chat_area.width {
        tui_state.persisted_chat_rows = 0;
        tui_state.persisted_chat_width = chat_area.width;
    }

    let segments = build_segments(shadow_engine, tui_state);
    let total_rows = total_segment_rows(&segments, chat_area.width);
    let persisted_target = if include_visible_viewport {
        total_rows
    } else {
        total_rows.saturating_sub(chat_area.height as usize)
    };

    if persisted_target < tui_state.persisted_chat_rows {
        tui_state.persisted_chat_rows = 0;
    }

    let mut next_row = tui_state.persisted_chat_rows;
    let mut pending_rows = persisted_target.saturating_sub(tui_state.persisted_chat_rows);

    while pending_rows > 0 {
        let chunk_rows = pending_rows.min(u16::MAX as usize) as u16;
        terminal.insert_before(chunk_rows, |buf| {
            render_chat_slice(buf, buf.area, tui_state, shadow_engine, &segments, next_row);
        })?;
        next_row += chunk_rows as usize;
        pending_rows -= chunk_rows as usize;
    }

    tui_state.persisted_chat_rows = persisted_target;
    Ok(())
}

fn build_segments(shadow_engine: &ShadowEngine, tui_state: &TuiAppState) -> Vec<Segment> {
    let mut segments = Vec::new();

    for (msg_idx, msg) in shadow_engine.messages.iter().enumerate() {
        match &msg.kind {
            MessageKind::MemoryTree(tree) => {
                let lines = tree_to_lines(
                    tree,
                    tui_state.memory_focus == Some(msg_idx),
                    tui_state.memory_focus == Some(msg_idx) && tui_state.memory_edit_mode,
                );
                segments.push(Segment::Lines(lines));
                segments.push(Segment::Lines(vec![Line::from("")]));
            }
            _ => {
                let mut lines = message_to_lines(msg, tui_state.tick);
                if msg.indent == 0 {
                    lines.push(Line::from(""));
                }
                segments.push(Segment::Lines(lines));
            }
        }
    }

    segments
}

fn memory_cursor_row(
    focus_idx: usize, shadow_engine: &ShadowEngine, tick: u64, available_width: u16,
) -> Option<usize> {
    let mut row = 0usize;

    for (msg_idx, message) in shadow_engine.messages.iter().enumerate() {
        match &message.kind {
            MessageKind::MemoryTree(tree) => {
                if msg_idx == focus_idx {
                    return Some(row + tree_cursor_screen_row(tree, available_width));
                }
                row += tree_render_height(tree, available_width) as usize + 1;
            }
            _ => {
                let mut lines = message_to_lines(message, tick);
                if message.indent == 0 {
                    lines.push(Line::from(""));
                }
                row += lines_height(&lines, available_width);
            }
        }
    }

    None
}

fn render_chat_slice(
    buf: &mut Buffer, area: Rect, _tui_state: &TuiAppState, _shadow_engine: &mut ShadowEngine,
    segments: &[Segment], start_row: usize,
) {
    let mut virtual_y = 0usize;
    let mut screen_y = area.top();

    for segment in segments {
        if screen_y >= area.bottom() {
            break;
        }

        let seg_height = segment_height(segment, area.width);
        let seg_end = virtual_y + seg_height;

        if seg_end <= start_row {
            virtual_y = seg_end;
            continue;
        }

        let skip = start_row.saturating_sub(virtual_y).min(seg_height);
        let remaining_screen = (area.bottom() - screen_y) as usize;
        let visible_count = seg_height.saturating_sub(skip).min(remaining_screen);

        if visible_count == 0 {
            break;
        }

        match segment {
            Segment::Lines(lines) => {
                let seg_rect = Rect::new(area.left(), screen_y, area.width, visible_count as u16);
                Paragraph::new(lines.clone())
                    .wrap(Wrap { trim: false })
                    .scroll((skip as u16, 0))
                    .render(seg_rect, buf);
            }
        }

        screen_y += visible_count as u16;
        virtual_y = seg_end;
    }
}

fn total_segment_rows(segments: &[Segment], available_width: u16) -> usize {
    segments
        .iter()
        .map(|segment| segment_height(segment, available_width))
        .sum()
}

fn segment_height(segment: &Segment, available_width: u16) -> usize {
    match segment {
        Segment::Lines(lines) => lines_height(lines, available_width),
    }
}

fn lines_height(lines: &[Line<'static>], available_width: u16) -> usize {
    let width = available_width.max(1);
    let sentinel_style = Style::default()
        .fg(Color::Rgb(1, 2, 3))
        .bg(Color::Rgb(1, 2, 3));

    let mut rendered_lines = lines.to_vec();
    rendered_lines.push(Line::from(vec![Span::styled("█", sentinel_style)]));

    let upper_bound = lines
        .iter()
        .map(|line| {
            let line_width = line.width().max(1);
            line_width.div_ceil(width as usize)
        })
        .sum::<usize>()
        .saturating_add(1)
        .max(1);

    let area = Rect::new(0, 0, width, upper_bound.min(u16::MAX as usize) as u16);
    let mut temp_buf = Buffer::empty(area);
    Paragraph::new(rendered_lines)
        .wrap(Wrap { trim: false })
        .render(area, &mut temp_buf);

    for row in 0..area.height {
        for col in 0..area.width {
            let cell = &temp_buf[(col, row)];
            if cell.symbol() == "█" && cell.style().fg == Some(Color::Rgb(1, 2, 3)) {
                return row as usize;
            }
        }
    }

    upper_bound.saturating_sub(1)
}

fn chat_area(total_area: Rect, tui_state: &TuiAppState) -> Rect {
    let bottom_height = if tui_state.slash_mode {
        SLASH_COMMANDS.len() as u16
    } else {
        1
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(bottom_height),
        ])
        .split(total_area);

    chunks[0]
}

fn visible_top(total_rows: usize, viewport_rows: usize, tui_state: &TuiAppState) -> usize {
    let max_scroll = total_rows.saturating_sub(viewport_rows);
    if tui_state.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(tui_state.scroll_offset.min(max_scroll))
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
                    .insert(0, Span::styled(format!("{}❯  ", pad), default()));
            }
            lines
        }
        MessageKind::Tool(tool) => tool_to_lines(tool, &pad, tick),
        // MemoryTree is handled upstream in render_chat, never reaches here
        MessageKind::MemoryTree(_) => vec![],
    }
}

fn render_session_list(
    f: &mut Frame, area: Rect, tui_state: &TuiAppState, shadow_engine: &mut ShadowEngine,
) {
    let history_sessions = match shadow_engine.list_sessions(30) {
        Ok(sessions) => sessions,
        Err(e) => {
            let line = Line::from(Span::styled(
                format!("  failed to load sessions: {}", e),
                error_style(),
            ));
            f.render_widget(Paragraph::new(line), area);
            return;
        }
    };

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
                        format!(
                            "{:<42} {}",
                            title,
                            format_timestamp(&session.created_at_ms.to_string())
                        ),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!(
                            "{:<42} {}",
                            title,
                            format_timestamp(&session.created_at_ms.to_string())
                        ),
                        dim(),
                    ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_line_height_expands_for_long_lines() {
        let lines = vec![Line::from("123456789")];
        assert_eq!(lines_height(&lines, 4), 3);
    }

    #[test]
    fn lines_height_counts_blank_lines() {
        let lines = vec![Line::from("abc"), Line::from("")];
        assert_eq!(lines_height(&lines, 10), 2);
    }

    #[test]
    fn visible_top_uses_scroll_offset_when_auto_scroll_disabled() {
        let mut state = TuiAppState::default();
        state.auto_scroll = false;
        state.scroll_offset = 3;

        assert_eq!(visible_top(20, 5, &state), 12);
    }

    #[test]
    fn visible_top_stays_at_latest_when_auto_scroll_enabled() {
        let state = TuiAppState::default();
        assert_eq!(visible_top(20, 5, &state), 15);
    }
}
