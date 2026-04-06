mod tui;

use db::Database;
use llm::LlmClient;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::CrosstermBackend;
use shadow_core::engine::ShadowEngine;
use shadow_core::setup;
use shadow_core::*;
use std::env;
use std::io;
use std::sync::Arc;
use tracing_subscriber;
use tui::run;

#[tokio::main]
#[hotpath::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let (config, paths) = setup::run_setup()?;
    let db_conn = Arc::new(Database::init(&paths.db)?);

    let llm_client = Arc::new(
        LlmClient::init(&config)
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?,
    );

    let mut shadow_engine = ShadowEngine::new(db_conn, llm_client, config, paths)?;

    crossterm::terminal::enable_raw_mode()?;
    let terminal_height = crossterm::terminal::size()?.1;
    let viewport = configured_viewport(terminal_height);

    let terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions { viewport },
    )?;

    let result = run(terminal, &mut shadow_engine).await;
    crossterm::terminal::disable_raw_mode()?;
    result?;

    println!("");
    Ok(())
}

fn configured_viewport(terminal_height: u16) -> Viewport {
    match env::var("SHADOW_TUI_VIEWPORT").ok().as_deref() {
        Some("fullscreen") => Viewport::Fullscreen,
        _ => Viewport::Inline(default_inline_height(terminal_height)),
    }
}

fn default_inline_height(terminal_height: u16) -> u16 {
    let reserved_rows = if terminal_height >= 20 {
        4
    } else if terminal_height >= 12 {
        2
    } else {
        1
    };

    terminal_height.saturating_sub(reserved_rows).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_inline_height_leaves_rows_above_viewport() {
        assert_eq!(default_inline_height(24), 20);
        assert_eq!(default_inline_height(16), 14);
        assert_eq!(default_inline_height(8), 7);
    }
}
