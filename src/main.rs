mod db;
mod ingest;
mod models;

use db::Database;
use ingest::file_ingest;
use models::RawLog;

use rusqlite::Result;

fn main() -> Result<()> {
    let conn = Database::new("data/shadow_logs.db").unwrap();
    let dir = dirs::home_dir()
        .unwrap()
        .join("Library/Mobile Documents/com~apple~CloudDocs/ShadowLogs/");

    let _ = file_ingest(&conn, &dir);

                }
            };
        }
        
        Commands::Log { content } => {
            match content {
                Some(new_log)  => {
                    info!("New Log {}", new_log );

                }
                None => {
                    info!("No Log");

    Ok(())
}
