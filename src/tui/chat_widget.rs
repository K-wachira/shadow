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

