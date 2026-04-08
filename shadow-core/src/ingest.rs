use crate::db::Database;
use crate::db::EntryLog;
use crate::db::RawLog;

use std::fs;
use std::path::PathBuf;

pub fn process_json_file(log_name: &String, dir: &PathBuf) -> Result<RawLog, String> {
    let complete_path =
        dirs::home_dir()
            .unwrap()
            .join(format!("{}{}", &dir.to_string_lossy(), &log_name));

    let content = std::fs::read_to_string(complete_path).map_err(|e| e.to_string())?;
    let raw: RawLog = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(raw)
}

pub fn file_ingest(conn: &Database, path: &PathBuf) -> color_eyre::Result<Vec<EntryLog>> {
    let mut ingested = vec![];
    let expanded_path = dirs::home_dir()
        .map(|h| {
            PathBuf::from(
                path.to_string_lossy()
                    .replacen("~", &h.to_string_lossy(), 1),
            )
        })
        .unwrap_or_else(|| path.clone());
    match get_files(&expanded_path) {
        Ok(files) => {
            for file_name in files {
                if !*&file_name.contains(&".json".to_string()) || file_name.starts_with(".") {
                    continue;
                };
                if let Ok(Some(log)) = conn.insert_file_ingest(&file_name, &expanded_path) {
                    ingested.push(log);
                }
            }
        }
        Err(e) => {
            tracing::error!("Log ingestion failed: {}", e);
        }
    }
    Ok(ingested)
}

pub fn get_files(dir: &PathBuf) -> Result<Vec<String>, String> {
    let files: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    Ok(files)
}
