mod ask;
mod commands;
mod db;
mod handlers;
mod ingest;
mod models;
mod ollama;
mod tui;
mod run_layout;
mod shadow_layout;

use ask::ask;
use clap::Parser;
use commands::Args;
use commands::Commands;
use db::Database;
use handlers::handle_ask;
use handlers::handle_ingests;
use handlers::handle_log;
use handlers::handle_recent;
use handlers::handle_stats;
use ollama::LlmClient;
use tracing_subscriber;
use std::sync::Arc;
use tui::run;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let db_conn = Database::new("data/shadow.db")?;
    let ollama_conn = Arc::new(LlmClient::init().map_err(|e| color_eyre::eyre::eyre!(e))?);
    
    let args = Args::parse();

    match args.command {
        Some(Commands::Ask { query }) => handle_ask(&db_conn, ollama_conn, query).await,
        Some(Commands::Recent { content }) => handle_recent(content, &db_conn).await,
        Some(Commands::Ingest) => handle_ingests(&db_conn),
        Some(Commands::Log { content }) => handle_log(content).await,
        Some(Commands::Stats) => handle_stats().await,
        None => {
            // no command passed → launch TUI
            let terminal = ratatui::init();
            let result = run(terminal, ollama_conn, &db_conn).await;
            ratatui::restore();
            result?;
        }
    }

    Ok(())
}
