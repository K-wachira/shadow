mod db;
mod ingest;
mod models;

use db::Database;
use ingest::file_ingest;
use models::RawLog;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    let db_conn = Database::new("data/shadow_logs.db").map_err(|e| color_eyre::eyre::eyre!(e))?;
    let ollama_conn = LlmClient::init().map_err(|e| color_eyre::eyre::eyre!(e))?;
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
            let result = run(terminal, ollama_conn);
            ratatui::restore();
            return result.await;
        }
    }

    Ok(())
}
