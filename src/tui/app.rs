// use color_eyre::Error;
// use crossterm::event::{self, Event};
// use ratatui::{DefaultTerminal, Frame};

use crossterm::ExecutableCommand;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui_textarea::TextArea;
use std::cmp;
use std::io::stdout;
use tokio_stream::StreamExt;

pub enum Role {
    User,
    Assistant,
}

pub struct App {
    pub input: String, // what the user is typing
    // pub messages: Vec<String>,
    pub messages: Vec<(Role, String)>, // conversation history
    pub context_logs: Vec<String>,     // logs shown in right panel
    pub model: String,                 // "gemma3:12b"
    pub last_log: String,              // "24 Feb @ Nairobi"
}

use crate::ollama::LlmClient;

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: vec![],
            context_logs: vec![],
            model: String::from("gemma3:12b"),
            last_log: String::from(""),
        }
    }
}

pub async fn run(mut terminal: DefaultTerminal, ollama_conn: LlmClient) -> color_eyre::Result<()> {
    let mut app = App::new();
    let mut textarea = TextArea::default();
    textarea.set_block(Block::bordered().title(" input "));
    let style = Style::default().add_modifier(Modifier::SLOW_BLINK);
    textarea.set_cursor_line_style(style);

    loop {
        stdout().execute(SetCursorStyle::BlinkingBar)?;
        terminal.draw(|frame| render(frame, &app, &mut textarea))?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break,
                KeyCode::Enter => {
                    let query = textarea.lines()[0].clone();
                    app.messages.push((Role::User, query.clone()));
                    textarea = TextArea::default();
                    textarea.set_block(Block::bordered().title(" input "));

                    // call ollama directly
                    if let Ok(mut stream) = ollama_conn.ollama_ask_stream(&query).await {
                        let mut response = String::new();
                        while let Some(chunk) = stream.next().await {
                            if let Ok(res) = chunk {
                                for r in res {
                                    response.push_str(&r.response);
                                }
                            }
                        }
                        app.messages.push((Role::Assistant, response));
                    }
                }
                _ => {
                    textarea.input(key); // let textarea handle everything
                }
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, app: &App, textarea: &mut TextArea) {
    let area = frame.area();
    let textarea_len = cmp::min(6, textarea.lines().len() as u16 + 2);

    // Vertical split: header / middle / input
    let outer = Layout::vertical([
        Constraint::Length(4),            // header
        Constraint::Min(0),               // middle
        Constraint::Length(textarea_len), // input
    ])
    .split(area);

    // Middle: context (left) + conversation (right)
    let middle = Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(outer[1]);

    // Header
    let header = Paragraph::new(" model: llama3.2          last log: 24 Feb @ Nairobi")
        .block(Block::bordered().title(" shadow "));
    frame.render_widget(header, outer[0]);

    // Context
    let conversation = Block::bordered().title(" context ");
    frame.render_widget(conversation, middle[0]);

    // Conversation
    let content = app
        .messages
        .iter()
        .map(|(role, text)| match role {
            Role::User => format!("Me > {}", text),
            Role::Assistant => format!("  {}", text),
        })
        .collect::<Vec<String>>()
        .join("\n");
    let context = Paragraph::new(content).block(Block::bordered().title(" conversation "));
    frame.render_widget(context, middle[1]);

    // Input
    textarea.set_block(Block::bordered().title(" input "));
    frame.render_widget(&*textarea, outer[2]);

    // frame.render_widget(&textarea, outer[2]);
}
