mod tui;

use db::Database;
use llm::LlmClient;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::CrosstermBackend;
use shadow_core::engine::ShadowEngine;
use shadow_core::*;
use std::env;
use std::io;
use std::io::IsTerminal;
use std::sync::Arc;
use tracing_subscriber;
use tui::run;

#[tokio::main]
#[hotpath::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

#[derive(Clone, Copy)]
enum ViewportMode {
    Auto,
    Inline,
    Fullscreen,
}

impl ViewportMode {
    fn from_env() -> Self {
        match env::var("SHADOW_TUI_VIEWPORT") {
            Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
                "auto" => Self::Auto,
                "inline" => Self::Inline,
                "fullscreen" | "full" => Self::Fullscreen,
                other => {
                    eprintln!(
                        "unknown SHADOW_TUI_VIEWPORT='{}'; expected auto|inline|fullscreen, using auto",
                        other
                    );
                    Self::Auto
                }
            },
            Err(_) => Self::Auto,
        }
    }
}

struct TerminalSession {
    raw_mode_enabled: bool,
    mouse_capture_enabled: bool,
}

impl TerminalSession {
    fn start() -> color_eyre::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
        Ok(Self {
            raw_mode_enabled: true,
            mouse_capture_enabled: true,
        })
    }

    fn cleanup(&mut self) {
        if self.mouse_capture_enabled {
            let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);
            self.mouse_capture_enabled = false;
        }
        if self.raw_mode_enabled {
            let _ = crossterm::terminal::disable_raw_mode();
            self.raw_mode_enabled = false;
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn is_cursor_position_read_error(err: &io::Error) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("cursor position could not be read")
}

fn build_terminal_inline(height: u16) -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )
}

fn build_terminal_fullscreen() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Fullscreen,
        },
    )
}

fn build_terminal(
    mode: ViewportMode,
) -> color_eyre::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    match mode {
        ViewportMode::Fullscreen => Ok(build_terminal_fullscreen()?),
        ViewportMode::Inline => {
            let height = crossterm::terminal::size()?.1;
            Ok(build_terminal_inline(height)?)
        }
        ViewportMode::Auto => {
            if !io::stdout().is_terminal() {
                eprintln!("stdout is not a TTY; using fullscreen viewport");
                return Ok(build_terminal_fullscreen()?);
            }

            let height = crossterm::terminal::size()?.1;
            match build_terminal_inline(height) {
                Ok(terminal) => Ok(terminal),
                Err(err) if is_cursor_position_read_error(&err) => {
                    eprintln!(
                        "inline viewport failed to read cursor position; falling back to fullscreen viewport"
                    );
                    Ok(build_terminal_fullscreen()?)
                }
                Err(err) => Err(err.into()),
            }
        }
    }
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let db_conn = Arc::new(Database::new("data/shadow.db")?);

    let model = "deepseek-r1:latest";
    let provider = "ollama";

    // let model = "Qwen/Qwen3-4B";
    // let provider = "mistralrs";

    let llm_client = Arc::new(
        LlmClient::init(provider, model)
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?,
    );

    let mut shadow_engine = ShadowEngine::new(db_conn, llm_client)?;
    let mut terminal_session = TerminalSession::start()?;
    let terminal = build_terminal(ViewportMode::from_env())?;

    let result = run(terminal, &mut shadow_engine).await;
    terminal_session.cleanup();
    result?;

    println!("");
    Ok(())
}
