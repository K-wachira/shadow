mod tui;

use shadow_core::*;
use db::Database;
use ollama::LlmClient;
use tracing_subscriber;
use std::sync::Arc;
use tui::run;
use shadow_core::engine::ShadowEngine;
use ratatui::{Terminal, TerminalOptions, Viewport};
use ratatui::backend::CrosstermBackend;
use std::io;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let db_conn = Arc::new(Database::new("data/shadow.db")?);
    let ollama_conn = Arc::new(LlmClient::init().map_err(|e| color_eyre::eyre::eyre!(e))?);
    let model = "";
    let mut shadow_engine = ShadowEngine::new(db_conn, ollama_conn, model)?;
    
    crossterm::terminal::enable_raw_mode()?;
    let height = crossterm::terminal::size()?.1;

    let terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height), // adjust height as needed
        },
    )?;
    
    let result = run(terminal,  &mut shadow_engine ).await;
    crossterm::terminal::disable_raw_mode()?;
    println!();
    result?;
    
    // let args = Args::parse();
    // match args.command {
    //     Some(Commands::Ask { query }) => handle_ask(&db_conn, &ollama_conn, query).await,
    //     Some(Commands::Recent { content }) => handle_recent(content, &db_conn).await,
    //     Some(Commands::Ingest) => handle_ingests(&db_conn),
    //     Some(Commands::Log { content }) => handle_log(content).await,
    //     Some(Commands::Stats) => handle_stats().await,
    //     None => {
    //         // no command passed → launch TUI
    //         let terminal = ratatui::init();
    //         let result = run(terminal,  &mut shadow_engine ).await;
    //         ratatui::restore();
    //         result?;
    //     }
    // }

    Ok(())
}
