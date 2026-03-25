use ratatui::{
    Frame,
    layout::{Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crate::tui::AppState;
use crate::tui::Message;
use crate::tui::MessageKind;
use crate::tui::ToolCall;
use crate::tui::ToolState;
use crate::tui::bright_bold;
use crate::tui::default;
use crate::tui::dim;
use crate::tui::markdown_to_lines;
use crate::tui::muted;
use crate::tui::very_dim;
use crate::tui::logo_lines;

pub fn render_chat(f: &mut Frame, area: Rect, state: &AppState) {
    if state.history_mode {
        render_session_list(f, area, state);
        return;
    }

    let mut all_lines: Vec<Line> = vec![];

    for msg in &state.messages {
        let mut lines = message_to_lines(msg, state.tick);
        all_lines.append(&mut lines);
        // Blank line after each top-level message for breathing room
        if msg.indent == 0 {
            all_lines.push(Line::from(""));
        }
    }

    // Scroll window: show bottom `area.height` lines, adjusted by scroll_offset
    let height = area.height as usize;
    let width = area.width as usize;

    let visual_lines: Vec<(usize, Line)> = all_lines
        .into_iter()
        .enumerate()
        .map(|(_, line)| {
            let len = line.width();
            let rows = if len == 0 {
                1
            } else {
                (len + width - 1) / width
            };
            (rows, line)
        })
        .collect();

    let total_visual: usize = visual_lines.iter().map(|(r, _)| r).sum();
    let max_scroll = total_visual.saturating_sub(height);

    let skip_visual = if state.auto_scroll {
        max_scroll
    } else {
        let offset = state.scroll_offset.min(max_scroll);
        max_scroll.saturating_sub(offset)
    };

    // now skip logical lines until we've passed skip_visual visual rows
    let mut skipped = 0;
    let visible: Vec<Line> = visual_lines
        .into_iter()
        .skip_while(|(rows, _)| {
            if skipped + rows <= skip_visual {
                skipped += rows;
                true
            } else {
                false
            }
        })
        .take(height)
        .map(|(_, line)| line)
        .collect();
    f.render_widget(Paragraph::new(visible).wrap(Wrap { trim: true }), area);
}

fn message_to_lines(msg: &Message, tick: u64) -> Vec<Line<'static>> {
    let pad = "  ".repeat(msg.indent as usize);

    match &msg.kind {
        MessageKind::Logo => logo_lines(),

        MessageKind::UserInput { text } => vec![Line::from(vec![
            Span::raw(format!("{}>  ", pad)),
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
                    .insert(0, Span::styled(format!("{}›  ", pad), default()));
            }
            lines
        }
        MessageKind::Tool(tool) => tool_to_lines(tool, &pad, tick),
    }
}

fn render_session_list(f: &mut Frame, area: Rect, state: &AppState) {
    if state.history_sessions.is_empty() {
        let line = Line::from(Span::styled("  no sessions found", dim()));
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let items: Vec<Line> = state
        .history_sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let date = chrono::DateTime::from_timestamp_millis(session.created_at_ms)
                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%d %b %Y").to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let title = if session.title.len() > 40 {
                format!("{}…", &session.title[..40])
            } else {
                session.title.clone()
            };

            if i == state.history_cursor {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<42} {}", title, date),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{:<42} {}", title, date), dim()),
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

