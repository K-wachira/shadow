use crate::models::RawLog;

use std::fs;
use std::path::PathBuf;

pub fn process_json_file(log_name: &String, dir: &PathBuf) -> color_eyre::Result<RawLog> {
    let complete_path =
        dirs::home_dir()
            .unwrap()
            .join(format!("{}{}", &dir.to_string_lossy(), &log_name));

    let content = std::fs::read_to_string(complete_path).map_err(|e| color_eyre::eyre::eyre!(e));
    let raw: RawLog = serde_json::from_str(&content?).map_err(|e| color_eyre::eyre::eyre!(e))?;
    Ok(raw)
}


pub fn get_files(dir: &PathBuf) -> color_eyre::Result<Vec<String>> {
    let files: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| color_eyre::eyre::eyre!(e))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    Ok(files)
}
