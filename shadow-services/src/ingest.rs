use crate::models::RawLog;

use std::fs;
use std::path::PathBuf;

pub fn process_json_file(log_name: &String, dir: &PathBuf) -> color_eyre::Result<RawLog> {
    let complete_path = dir.join(log_name);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_files_returns_files_in_directory() {
        let tmp = std::env::temp_dir().join("shadow_test_get_files");
        let _ = fs::create_dir_all(&tmp);
        let f1 = tmp.join("test1.txt");
        let f2 = tmp.join("test2.json");
        fs::write(&f1, "data1").unwrap();
        fs::write(&f2, "data2").unwrap();

        let files = get_files(&tmp).unwrap();
        assert!(files.contains(&"test1.txt".to_string()));
        assert!(files.contains(&"test2.json".to_string()));

        // Cleanup
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn get_files_returns_empty_for_empty_dir() {
        let tmp = std::env::temp_dir().join("shadow_test_empty_dir");
        let _ = fs::create_dir_all(&tmp);
        let files = get_files(&tmp).unwrap();
        assert!(files.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn get_files_returns_error_for_nonexistent_dir() {
        let result = get_files(&PathBuf::from(
            "/nonexistent/path/that/does/not/exist/12345",
        ));
        assert!(result.is_err());
    }

    #[test]
    fn process_json_file_parses_valid_json() {
        let tmp = std::env::temp_dir().join("shadow_test_process");
        let _ = fs::create_dir_all(&tmp);
        let json = r#"{
            "content": "Test log entry",
            "energy": 8,
            "mood": 7,
            "weather": "sunny",
            "location": "home",
            "time_stamp": "2026-01-15T10:00:00Z",
            "device": "iPhone",
            "type": "daily"
        }"#;
        let f = tmp.join("test_log.json");
        fs::write(&f, json).unwrap();

        let result = process_json_file(&"test_log.json".to_string(), &tmp);
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.content, "Test log entry");
        assert_eq!(raw.energy, Some(8));
        assert_eq!(raw.mood, Some(7));
        assert_eq!(raw.weather, Some("sunny".to_string()));
        assert_eq!(raw.location, Some("home".to_string()));
        assert_eq!(raw.time_stamp, "2026-01-15T10:00:00Z");
        assert_eq!(raw.device, Some("iPhone".to_string()));
        assert_eq!(raw.log_type, Some("daily".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn process_json_file_with_optional_fields() {
        let tmp = std::env::temp_dir().join("shadow_test_optional");
        let _ = fs::create_dir_all(&tmp);
        let json = r#"{
            "content": "Minimal log",
            "time_stamp": "2026-01-15T12:00:00Z"
        }"#;
        let f = tmp.join("minimal.json");
        fs::write(&f, json).unwrap();

        let result = process_json_file(&"minimal.json".to_string(), &tmp);
        assert!(result.is_ok());
        let raw = result.unwrap();
        assert_eq!(raw.content, "Minimal log");
        assert_eq!(raw.energy, None);
        assert_eq!(raw.weather, None);
        assert_eq!(raw.device, None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn process_json_file_returns_error_for_invalid_json() {
        let tmp = std::env::temp_dir().join("shadow_test_invalid");
        let _ = fs::create_dir_all(&tmp);
        fs::write(tmp.join("bad.json"), "not valid json{{{").unwrap();

        let result = process_json_file(&"bad.json".to_string(), &tmp);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn process_json_file_returns_error_for_missing_file() {
        let tmp = std::env::temp_dir().join("shadow_test_missing");
        let _ = fs::create_dir_all(&tmp);

        let result = process_json_file(&"nonexistent.json".to_string(), &tmp);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
