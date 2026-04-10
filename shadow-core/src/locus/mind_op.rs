use shadow_services::models::EntryLog;
use crate::llm::ChatMessage;
use crate::llm::LlmClient;
use crate::setup::ShadowPaths;
use std::sync::Arc;
use std::path::PathBuf;
use shadow_continuity::mind::ShadowMind;
use shadow_continuity::mind;
use shadow_continuity::mind::Belief;
use crate::db::Database;
use shadow_services::ingest::get_files;
const LOG_LIMIT: i32 = 30;

pub async fn reflect(
    llm_client: Arc<LlmClient>,
    paths: ShadowPaths,
    current_mind: ShadowMind,
    logs_json: String
) -> color_eyre::Result<ShadowMind> {
    let skill = std::fs::read_to_string(&paths.mind_skill)?;
    let today = chrono::Local::now().format("%Y-%m-%d");
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: skill,
            ..ChatMessage::default()
        },
        ChatMessage::user(format!(
            "--- Current shadow.mind ---\n{current_mind:?}\n\n\
            --- Recent Logs ---\n{logs_json}\n\n\
            --- Today's Date ---\n{today}\n\n\
            ---\nProduce the new shadow.mind. Start your response with {{ and end with }}. No code fences, no markdown, no explanation."
        )),
        ChatMessage {
            role: "assistant".into(),
            content: "{".into(),
            ..ChatMessage::default()
        },
    ];

    let response = llm_client
        .llm_ask(&messages)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let full = format!("{{{}", response.trim());
    let new_mind: ShadowMind = json5::from_str(&full)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse shadow.mind: {e}\nRaw:\n{full}"))?;

    mind::save(&new_mind, &paths.mind)?;
    Ok(new_mind)
}

// spawnable — no db access
pub async fn reflect_with_input( llm_client: Arc<LlmClient>,
    paths: ShadowPaths,
    db: &Database
 ) ->  color_eyre::Result<ShadowMind> {
    let skill = std::fs::read_to_string(&paths.mind_skill)?;
    let logs_json = gather_reflect_input(db)?;
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: skill,
            ..ChatMessage::default()
        },
        ChatMessage::user(format!(
            "--- Current shadow.mind ---\n{{&self.current_mind}}\n\n--- Recent Logs ---\n{logs_json}\n\n---\nProduce the new shadow.mind. Output raw JSON5 only. No markdown. No explanation."
        )),
    ];

    let response = llm_client
        .llm_ask(&messages)
        .await
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let new_mind: ShadowMind = json5::from_str(&response)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to parse shadow.mind: {}", e))?;

    mind::save(&new_mind, &paths.mind)?;
    Ok(new_mind)
}

// called from main thread — fetches logs before spawning
pub fn gather_reflect_input( db: &Database) ->  color_eyre::Result<String> {
    let logs = db.get_logs(Some(LOG_LIMIT))?;
    let logs_json = serde_json::to_string_pretty(&logs)?;
    Ok(logs_json)
}

pub async fn process_ingested_logs(
    logs: Vec<EntryLog>,
    mind: ShadowMind,
    llm: Arc<LlmClient>,
    mind_path: PathBuf,
) {
    for log in logs {
        match extract_affected_fields(&mind, &log, &llm).await {
            Ok(fields) => {
                let mut updated_mind = mind.clone();
                for field_path in fields {
                    let parts: Vec<&str> = field_path.splitn(2, '.').collect();
                    if parts.len() != 2 { continue; }
                    let (layer, key) = (parts[0], parts[1]);

                    let belief = match layer {
                        "surface" => updated_mind.surface.get(key).cloned(),
                        "behavioural" => updated_mind.behavioural.get(key).cloned(),
                        "mental_model" => updated_mind.mental_model.get(key).cloned(),
                        "values" => updated_mind.values.get(key).cloned(),
                        _ => None,
                    };

                    if let Some(current) = belief {
                        match update_belief(&field_path, &current, &log, &llm).await {
                            Ok(updated) => {
                                match layer {
                                    "surface" => { updated_mind.surface.insert(key.to_string(), updated); }
                                    "behavioural" => { updated_mind.behavioural.insert(key.to_string(), updated); }
                                    "mental_model" => { updated_mind.mental_model.insert(key.to_string(), updated); }
                                    "values" => { updated_mind.values.insert(key.to_string(), updated); }
                                    _ => {}
                                }
                            }
                            Err(e) => tracing::error!("update failed for {}: {}", field_path, e),
                        }
                    }
                }
                if let Err(e) = mind::save(&updated_mind, &mind_path) {
                    tracing::error!("failed to save mind: {}", e);
                }
            }
            Err(e) => tracing::error!("extraction failed for log {}: {}", log.id, e),
        }
    }
}


pub async fn extract_affected_fields(
    mind: &ShadowMind,
    log: &EntryLog,
    llm: &LlmClient,
) -> color_eyre::Result<Vec<String>> {
    let fields = mind::collect_field_paths(mind);
    let log_str = serde_json::to_string_pretty(log).unwrap_or_default();
    let prompt = mind::build_extraction_prompt(log_str, &fields);
    let messages = vec![ChatMessage::user(prompt)];
    let response = llm.llm_ask(&messages).await?;
    mind::parse_field_array(&response)
}

pub async fn update_belief(
    field_path: &str,
    belief: &Belief,
    log: &EntryLog,
    llm: &LlmClient,
) -> color_eyre::Result<Belief> {
    let prompt = mind::build_update_prompt(field_path, belief, log);
    let messages = vec![ChatMessage::user(prompt)];
    let response = llm.llm_ask(&messages).await?;

    let clean = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    Ok(serde_json::from_str(clean)?)
}


pub fn ingest_icloud_logs(
    db: &Database,
    source_path: &PathBuf,
) -> color_eyre::Result<Vec<EntryLog>> {
    let mut ingested = vec![];
    let expanded_path = dirs::home_dir()
        .map(|h| {
            PathBuf::from(
                &source_path.to_string_lossy()
                    .replacen("~", &h.to_string_lossy(), 1),
            )
        })
        .unwrap_or_else(|| source_path.to_path_buf());

    match get_files(&expanded_path) {
        Ok(files) => {
            for file_name in files {
                if !file_name.contains(".json") || file_name.starts_with('.') {
                    continue;
                }
                if let Ok(Some(log)) = db.insert_file_ingest(&file_name, &expanded_path) {
                    ingested.push(log);
                }
            }
        }
        Err(e) => tracing::error!("Log ingestion failed: {}", e),
    }

    Ok(ingested)
}