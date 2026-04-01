mod tui;

use shadow_core::*;
use db::Database;
use llm::LlmClient;
use tracing_subscriber;
use std::sync::Arc;
use tui::run;
use shadow_core::engine::ShadowEngine;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::CrosstermBackend;
use std::io;

#[tokio::main]
#[hotpath::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let db_conn = Arc::new(Database::new("data/shadow.db")?);
    
    

    let model = "deepseek-r1:latest";
    let provider = "ollama";
    
    // let model = "Qwen/Qwen3-4B";
    // let provider = "mistralrs";
    
    let llm_client = Arc::new(
        LlmClient::init(provider, model).await
            .map_err(|e| color_eyre::eyre::eyre!(e))?
    );
    
    let mut shadow_engine = ShadowEngine::new(db_conn, llm_client)?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
    let height = crossterm::terminal::size()?.1;
     
    let terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height), // adjust height as needed
        },
    )?;
    
    let result = run(terminal,  &mut shadow_engine ).await;
    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
    crossterm::terminal::disable_raw_mode()?;
    result?;


    Ok(())
}
