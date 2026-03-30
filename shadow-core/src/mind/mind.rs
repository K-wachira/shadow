use std::path::PathBuf;
use color_eyre::Result;
use chrono::Utc;
use crate::mind::mind_model::ShadowMind;
use crate::mind::mind_model::Meta;
use std::collections::HashMap;
use crate::db::Database;
use std::sync::Arc;
use crate::llm::LlmClient;
use serde_json;

const MIND_SKILL_PATH: &str = "shadow-core/skill.md/mind_skill.md";
const MIND_PATH: &str = "shadow-core/data/shadowmind.json5";
const LOG_LIMIT: i32 = 30;

// called from main thread — fetches logs before spawning
pub fn gather_reflect_input(db: &Arc<Database>) -> Result<(String, String)> {
    let current_mind = std::fs::read_to_string(mind_path())
        .unwrap_or_else(|_| String::from("{}"));
    let logs = db.get_logs(Some(LOG_LIMIT))?;
    let logs_json = serde_json::to_string_pretty(&logs)?;
    Ok((current_mind, logs_json))
}

// spawnable — no db access
pub async fn reflect_with_input(llm_client: &Arc<LlmClient>, current_mind: String, logs_json: String) -> Result<ShadowMind> {
    let skill = std::fs::read_to_string(MIND_SKILL_PATH)?;

    let prompt = format!(
        "{skill}\n\n--- Current shadow.mind ---\n{current_mind}\n\n--- Recent Logs ---\n{logs_json}\n\n---\nProduce the new shadow.mind. Output raw JSON5 only. No markdown. No explanation.",
    );

    let response = llm_client.llm_ask(&prompt).await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let new_mind: ShadowMind = json5::from_str(&response)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse shadow.mind: {}", e))?;

    save(&new_mind)?;
    Ok(new_mind)
}



pub fn mind_path() -> PathBuf {
    PathBuf::from(MIND_PATH)
}

pub fn load() -> Result<ShadowMind> {
    let path = mind_path();
    if !path.exists() {
        return Ok(init());
    }
    let contents = std::fs::read_to_string(&path)?;
    let mind: ShadowMind = json5::from_str(&contents)?;
    Ok(mind)
}

pub fn save(mind: &ShadowMind) -> Result<()> {
    let path = mind_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = json5::to_string(mind)?;
    std::fs::write(&path, contents)?;
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
    Utc::now().format("%Y-%m-%d").to_string()
}