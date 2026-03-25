use crate::db::Database;
use crate::db::RawLog;

use std::fs;
use std::path::PathBuf;
use tracing::{ error };

pub fn process_json_file(log_name: &String, dir: &PathBuf) -> Result<RawLog, String> {
    let complete_path = dirs::home_dir()
            .unwrap()
            .join(format!("{}{}", &dir.to_string_lossy(), &log_name));
    
    let content = std::fs::read_to_string(complete_path).map_err(|e| e.to_string())?;
    let raw: RawLog = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(raw)
}

pub fn file_ingest(conn: &Database, dir: &PathBuf) -> Result<(), String> {
    match get_files(&dir) {
        Ok(files) => {
            for file_name in files {
                if !*&file_name.contains(&".json".to_string()) || file_name.starts_with(".")   {
                    continue
                };
                let _ = conn.insert_file_ingest(&file_name, &dir);
                // let _ = conn.insert(&process_json_file(&file_name, &dir)?);
            }
        }
        Err(e) => error!("Failed: {}", e),
    }
    Ok(())
}

pub fn get_files(dir: &PathBuf) -> Result<Vec<String>, String> {
    let files: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    Ok(files)
}
