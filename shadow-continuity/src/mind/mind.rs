use shadow_utils::utils::format_timestamp;
use crate::mind::mind_model::Meta;
use crate::mind::mind_model::ShadowMind;
use chrono::Utc;
use color_eyre::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn load(mind_path: &PathBuf) -> Result<ShadowMind> {
    if !mind_path.exists() {
        return Ok(init());
    }
    let contents = std::fs::read_to_string(&mind_path)?;
    let mind: ShadowMind = json5::from_str(&contents)?;
    Ok(mind)
}

pub fn save(mind: &ShadowMind, mind_path: &PathBuf) -> Result<()> {
    if let Some(parent) = mind_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = json5::to_string(mind)?;
    std::fs::write(&mind_path, contents)?;
    Ok(())
}

pub fn init() -> ShadowMind {
    ShadowMind {
        meta: Meta {
            version: 1,
            last_updated: today(),
            log_range: None,
            total_logs_considered: 0,
            rewrite_trigger: String::from("init"),
        },
        surface: HashMap::new(),
        behavioural: HashMap::new(),
        mental_model: HashMap::new(),
        values: HashMap::new(),
    }
}

pub fn today() -> String {
    format_timestamp(Utc::now().timestamp_millis().to_string().as_ref())
}
