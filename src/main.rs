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

    // Fetch and display
    // for file in conn.get_file_ingests(None)? {
    //     println!("File: {:?} {:?}", file.id, file.file_name);
    // }

    // for log in conn.get_logs(None)?{
    //     println!("Log: {:?}",log);
    // }

    Ok(())
}
