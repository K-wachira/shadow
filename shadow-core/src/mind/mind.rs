use crate::db::Database;
use crate::llm::LlmClient;
use crate::mind::mind_model::Meta;
use crate::mind::mind_model::ShadowMind;
use crate::setup::ShadowPaths;
use crate::utils::format_timestamp;
use chrono::Utc;
use color_eyre::Result;
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::llm::ChatMessage;

const LOG_LIMIT: i32 = 30;

// called from main thread — fetches logs before spawning
pub fn gather_reflect_input(db: &Arc<Database>, paths: &ShadowPaths) -> Result<(String, String)> {
    let current_mind = std::fs::read_to_string(&paths.mind).unwrap_or_else(|_| String::from("{}"));
    let logs = db.get_logs(Some(LOG_LIMIT))?;
    let logs_json = serde_json::to_string_pretty(&logs)?;
    Ok((current_mind, logs_json))
}

// spawnable — no db access
pub async fn reflect_with_input(
    llm_client: &Arc<LlmClient>, current_mind: String, logs_json: String, paths: &ShadowPaths,
) -> Result<ShadowMind> {
    let skill = std::fs::read_to_string(&paths.mind_skill)?;
    
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: skill,
        },
        ChatMessage {
            role: "user".into(),
            content: format!(
                "--- Current shadow.mind ---\n{current_mind}\n\n--- Recent Logs ---\n{logs_json}\n\n---\nProduce the new shadow.mind. Output raw JSON5 only. No markdown. No explanation."
            ),
        },
    ];

    let response = llm_client
        .llm_ask(&messages)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let new_mind: ShadowMind = json5::from_str(&response)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse shadow.mind: {}", e))?;

    save(&new_mind, &paths.mind)?;
    Ok(new_mind)
}

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
