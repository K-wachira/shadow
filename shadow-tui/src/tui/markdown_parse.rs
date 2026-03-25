use pulldown_cmark::Event;
use pulldown_cmark::HeadingLevel;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::TagEnd;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

pub fn markdown_to_lines(markdown: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut in_table_header = false;
    let mut table_cells: Vec<String> = Vec::new();
    let mut list_depth: usize = 0;
    let mut ordered_counters: Vec<u64> = Vec::new();

    let current_style = |stack: &Vec<Style>| -> Style { stack.last().copied().unwrap_or_default() };

    for event in parser {
        match event {
            // ── Text ────────────────────────────────────────────────────────
            Event::Text(text) => {
                let text = text.to_string();
                if in_code_block {
                    // preserve internal newlines in code blocks
                    for (i, part) in text.split('\n').enumerate() {
                        if i > 0 {
                            flush_line(&mut current_spans, &mut lines);
                        }
                        if !part.is_empty() {
                            current_spans.push(Span::styled(
                                part.to_string(),
                                Style::default().fg(Color::Green),
                            ));
                        }
                    }
                } else if in_table_header {
                    table_cells.push(text);
                } else {
                    current_spans.push(Span::styled(text, current_style(&style_stack)));
                }
            }

            // ── Inline code ─────────────────────────────────────────────────
            Event::Code(text) => {
                current_spans.push(Span::styled(
                    format!(" {} ", text.to_string()),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }

            // ── Code block ──────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::raw(""));
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_line(&mut current_spans, &mut lines);
                in_code_block = false;
                lines.push(Line::raw(""));
            }

            // ── Headings ────────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                let style = match level {
                    HeadingLevel::H1 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H2 => Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H3 => Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                };
                style_stack.push(style);
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::raw(""));
            }

            // ── Bold ────────────────────────────────────────────────────────
            Event::Start(Tag::Strong) => {
                let new = current_style(&style_stack).add_modifier(Modifier::BOLD);
                style_stack.push(new);
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }

            // ── Italic ──────────────────────────────────────────────────────
            Event::Start(Tag::Emphasis) => {
                let new = current_style(&style_stack).add_modifier(Modifier::ITALIC);
                style_stack.push(new);
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }

            // ── Strikethrough ───────────────────────────────────────────────
            Event::Start(Tag::Strikethrough) => {
                let new = current_style(&style_stack).add_modifier(Modifier::CROSSED_OUT);
                style_stack.push(new);
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }

            // ── Links — render text, drop URL ────────────────────────────────
            Event::Start(Tag::Link { .. }) => {
                let new = current_style(&style_stack)
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED);
                style_stack.push(new);
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }

            // ── Images — show alt text ───────────────────────────────────────
            Event::Start(Tag::Image { .. }) => {
                current_spans.push(Span::styled(
                    "[img] ".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::End(TagEnd::Image) => {}

            // ── Paragraphs ──────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::raw(""));
            }

            // ── Lists ───────────────────────────────────────────────────────
            Event::Start(Tag::List(ordered)) => {
                list_depth += 1;
                if let Some(start) = ordered {
                    ordered_counters.push(start);
                } else {
                    ordered_counters.push(0); // 0 = unordered marker
                }
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counters.pop();
                if list_depth == 0 {
                    lines.push(Line::raw(""));
                }
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let counter = ordered_counters.last_mut();
                let bullet = match counter {
                    Some(c) if *c > 0 => {
                        let n = *c;
                        *c += 1;
                        format!("{}{}. ", indent, n)
                    }
                    _ => format!("{}• ", indent),
                };
                current_spans.push(Span::styled(bullet, Style::default().fg(Color::DarkGray)));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut current_spans, &mut lines);
            }

            // ── Tables ──────────────────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                lines.push(Line::raw(""));
            }
            Event::End(TagEnd::Table) => {
                lines.push(Line::raw(""));
            }
            Event::Start(Tag::TableHead) => {
                in_table_header = true;
                table_cells.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_table_header = false;
                let header = table_cells
                    .iter()
                    .map(|c| format!(" {:<12}", c))
                    .collect::<Vec<_>>()
                    .join(" │");
                lines.push(Line::styled(
                    header,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ));
                let separator = table_cells
                    .iter()
                    .map(|_| "─".repeat(13))
                    .collect::<Vec<_>>()
                    .join("─┼");
                lines.push(Line::styled(
                    separator,
                    Style::default().fg(Color::DarkGray),
                ));
                table_cells.clear();
            }
            Event::Start(Tag::TableRow) => {
                table_cells.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_header {
                    let row = table_cells
                        .iter()
                        .map(|c| format!(" {:<12}", c))
                        .collect::<Vec<_>>()
                        .join(" │");
                    lines.push(Line::raw(row));
                }
            }
            Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {
                // text events inside cells are captured by in_table_header path
                // for body rows we need a separate flag
                if !in_table_header {
                    let text = current_spans
                        .drain(..)
                        .map(|s| s.content.to_string())
                        .collect::<String>();
                    table_cells.push(text);
                }
            }

            // ── Blockquote ──────────────────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                let new = current_style(&style_stack)
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC);
                style_stack.push(new);
                current_spans.push(Span::styled(
                    "▌ ".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                style_stack.pop();
                flush_line(&mut current_spans, &mut lines);
            }

            // ── Task list items ─────────────────────────────────────────────
            Event::TaskListMarker(checked) => {
                let marker = if checked { "☑ " } else { "☐ " };
                current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(if checked {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ));
            }

            // ── Horizontal rule ─────────────────────────────────────────────
            Event::Rule => {
                lines.push(Line::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::raw(""));
            }

            // ── Breaks ──────────────────────────────────────────────────────
            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                flush_line(&mut current_spans, &mut lines);
            }

            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

fn flush_line(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(spans.drain(..).collect::<Vec<_>>()));
    }
}