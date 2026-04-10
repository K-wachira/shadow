use crate::ask::ask;
use crate::config::Config;
use crate::db::Database;
use crate::db::SessionMessages;
use crate::db::Sessions;
use shadow_services::ingest::get_files;
use shadow_services::models::EntryLog;
use crate::llm::ChatMessage;
use crate::llm::LlmClient;
use crate::model::AssistantState;
use crate::model::Message;
use crate::model::MessageKind;
use crate::setup::ShadowPaths;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use std::path::PathBuf;
use shadow_continuity::mind::ShadowMind;
use shadow_continuity::mind;
use shadow_continuity::mind::Belief;

const LOG_LIMIT: i32 = 30;

pub struct ShadowEngine {
    pub db: Arc<Database>,
    pub llm_client: Arc<LlmClient>,
    pub session_id: i64,
    pub session_name: String,
    pub assistant_state: AssistantState,
    pub messages: Vec<Message>,
    pub mind: ShadowMind,
    pub config: Config,
    pub paths: ShadowPaths,
}

impl ShadowEngine {
    pub fn new(
        db: Arc<Database>, llm_client: Arc<LlmClient>, config: Config, paths: ShadowPaths,
    ) -> color_eyre::Result<Self> {
        let mind = mind::load(&paths.mind)?;
        Ok(Self {
            db,
            llm_client,
            session_id: 0,
            session_name: String::from("Untitled Session"),
            assistant_state: AssistantState::Idle,
            messages: vec![Message::logo(String::new())],
            mind,
            config,
            paths,
        })
    }

    fn start_session(&mut self) -> color_eyre::Result<()> {
        let session_id = self
            .db
            .create_session(&self.session_name, &self.llm_client.model_name)?;
        self.session_id = session_id;
        Ok(())
    }

    pub fn start_new_session(&mut self) {
        self.session_name = String::from("Untitled Session");
        self.session_id = 0;
        self.messages = vec![Message::logo(&self.llm_client.model_name)];
        self.assistant_state = AssistantState::Idle;
    }

    pub async fn send_message(
        &mut self, prompt: &str,
    ) -> color_eyre::Result<impl Stream<Item = String> + 'static> {
        // create session on first message
        if self.session_id == 0 {
            self.start_session()?;
        }

        self.messages.push(Message::user(prompt));
        self.db
            .insert_message(self.session_id, "user", prompt, None)?;
        self.assistant_state = AssistantState::Thinking { secs: 0 };

        let enriched =
            ask(&self.db, &self.messages, &self.paths).map_err(|e| color_eyre::eyre::eyre!(e))?;

        let llm_client = Arc::clone(&self.llm_client);

        let stream = async_stream::stream! {
            match llm_client.llm_ask_stream_with_tools(&enriched).await {
                Ok(mut s) => {
                    while let Some(chunk) = s.next().await {
                        yield chunk;
                    }
                }
                Err(e) => tracing::error!("llm_ask_stream error: {}", e),
            }
        };
        Ok(stream)
    }

    pub async fn reflect(
        llm_client: Arc<LlmClient>,
        paths: ShadowPaths,
        current_mind: ShadowMind,
        logs_json: String,
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

        tracing::error!("raw reflect response: {:?}", new_mind);
        mind::save(&new_mind, &paths.mind)?;
        Ok(new_mind)
    }
    
    // called from main thread — fetches logs before spawning
    pub fn gather_reflect_input(&mut self) ->  color_eyre::Result<String> {
        let logs = &self.db.get_logs(Some(LOG_LIMIT))?;
        let logs_json = serde_json::to_string_pretty(&logs)?;
        Ok(logs_json)
    }
    
    // spawnable — no db access
    pub async fn reflect_with_input( &mut self ) ->  color_eyre::Result<ShadowMind> {
        let skill = std::fs::read_to_string(&self.paths.mind_skill)?;
        let logs_json = &self.gather_reflect_input()?;
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
    
        let response = &self.llm_client
            .llm_ask(&messages)
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
    
        let new_mind: ShadowMind = json5::from_str(&response)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse shadow.mind: {}", e))?;
    
        mind::save(&new_mind, &self.paths.mind)?;
        Ok(new_mind)
    }
    
    pub async fn on_stream_complete(
        &mut self, response: &str, title_tx: mpsc::UnboundedSender<String>,
    ) -> color_eyre::Result<()> {
        self.db.insert_message(
            self.session_id,
            "assistant",
            response,
            Some(&self.llm_client.model_name),
        )?;
        self.assistant_state = AssistantState::Idle;

        if self.session_name == "Untitled Session" {
            let _ = self.spawn_title_generation(title_tx);
        }

        Ok(())
    }

    pub fn list_sessions(&self, limit: usize) -> color_eyre::Result<Vec<Sessions>> {
        self.db.get_recent_sessions(limit)
    }

    pub fn load_session(&mut self, session_id: i64) -> color_eyre::Result<Vec<SessionMessages>> {
        self.db.get_session_messages(session_id)
    }

    pub fn end_session(&self) -> color_eyre::Result<()> {
        self.db.end_session(self.session_id)
    }

    fn spawn_title_generation(&mut self, title_tx: mpsc::UnboundedSender<String>) {
        let llm_client = Arc::clone(&self.llm_client);

        // grab first user + assistant exchange
        let context: String = self
            .messages
            .iter()
            .filter_map(|m| match &m.kind {
                MessageKind::UserInput { text } => Some(format!("User: {}", text)),
                MessageKind::AssistantText { text } => Some(format!("Assistant: {}", text)),
                _ => None,
            })
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");

        tokio::spawn(async move {
            let messages = vec![ChatMessage::user(format!(
                "Generate a short session title (5 words max). Return only the title as plain text, nothing else.\n\n{}",
                context
            ))];
            match llm_client.llm_ask(&messages).await {
                Ok(title) => {
                    let _ = title_tx.send(title.trim().to_string());
                }
                Err(e) => {
                    tracing::error!("Title generation failed: {}", e);
                }
            }
        });
    }

    pub fn delete_current_session(&mut self) -> color_eyre::Result<()> {
        if self.session_id != 0 {
            self.db.delete_session(self.session_id)?;
        }
        self.session_id = 0;
        self.session_name = "Untitled Session".to_string();
        self.assistant_state = AssistantState::Idle;
        self.messages.clear();
        self.messages
            .push(Message::logo(&self.llm_client.model_name));
        Ok(())
    }

    pub fn ingest_icloud_logs(&self) -> color_eyre::Result<Vec<EntryLog>> {
        let mut ingested = vec![];
        let expanded_path = dirs::home_dir()
            .map(|h| {
                PathBuf::from(
                     &self.config.ingest.source_path.to_string_lossy()
                        .replacen("~", &h.to_string_lossy(), 1),
                )
            })
            .unwrap_or_else(|| self.config.ingest.source_path.clone());
        match get_files(&expanded_path) {
            Ok(files) => {
                for file_name in files {
                    if !*&file_name.contains(&".json".to_string()) || file_name.starts_with(".") {
                        continue;
                    };
                    if let Ok(Some(log)) = &self.db.insert_file_ingest(&file_name, &expanded_path) {
                        ingested.push(log.clone());
                    }
                }
            }
            Err(e) => {
                tracing::error!("Log ingestion failed: {}", e);
            }
        }
        Ok(ingested)
    }

    pub fn list_logs(&self, limit: Option<i32>) -> color_eyre::Result<Vec<EntryLog>> {
        self.db.get_logs(limit)
    }
    
    pub async fn process_ingested_logs(
        logs: Vec<EntryLog>,
        mind: ShadowMind,
        llm: Arc<LlmClient>,
        mind_path: PathBuf,
    ) {
        for log in logs {
            match ShadowEngine::extract_affected_fields(&mind, &log, &llm).await {
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
                            match ShadowEngine::update_belief(&field_path, &current, &log, &llm).await {
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
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::db::Database;
//     use crate::model::MessageKind;
//     use ollama_rs::Ollama;
//     use crate::llm::LlmProvider;

//     fn test_engine() -> ShadowEngine {
//         let db = Arc::new(Database::new(":memory:").expect("in-memory db"));
//         // LlmClient::init() only stores a URL — no actual connection is made here.
//         //
//         let model = "llama3.2";
//         let ollama = LlmProvider::Ollama(
//             Ollama::new("http://localhost".to_string(), 11434)
//         );
//         // let llm_conn = Arc::new(LlmClient::init(ollama, &model).map_err(|e| color_eyre::eyre::eyre!(e)).unwrap());

//         let model = "deepseek-r1:latest";
//         let provider = "ollama";
//         let llm_client = Arc::new(
//             LlmClient::init(provider, model).await
//                 .map_err(|e| color_eyre::eyre::eyre!(e)).unwrap()
//         );
//         ShadowEngine::new(db, llm_client, "test-model").expect("engine init")
//     }

//     // ── Construction ──────────────────────────────────────────────────────────

//     #[test]
//     fn new_engine_has_no_active_session() {
//         let engine = test_engine();
//         assert_eq!(engine.session_id, 0);
//         assert_eq!(engine.session_name, "Untitled Session");
//         assert_eq!(engine.model, "test-model");
//         assert!(!engine.assistant_state.is_active());
//     }

//     #[test]
//     fn new_engine_messages_contain_logo() {
//         let engine = test_engine();
//         assert_eq!(engine.messages.len(), 1);
//         assert!(matches!(engine.messages[0].kind, MessageKind::Logo));
//     }

//     // ── start_new_session ─────────────────────────────────────────────────────

//     #[test]
//     fn start_new_session_resets_state() {
//         let mut engine = test_engine();
//         // Manually set some state to verify it gets reset
//         engine.session_id = 42;
//         engine.session_name = "Old Name".to_string();
//         engine.messages.push(Message::user("hello"));
//         engine.assistant_state = AssistantState::Thinking { secs: 5 };

//         engine.start_new_session();

//         assert_eq!(engine.session_id, 0);
//         assert_eq!(engine.session_name, "Untitled Session");
//         assert_eq!(engine.messages.len(), 1);
//         assert!(matches!(engine.messages[0].kind, MessageKind::Logo));
//         assert!(!engine.assistant_state.is_active());
//     }

//     // ── list_sessions / load_session ──────────────────────────────────────────

//     #[test]
//     fn list_sessions_empty_when_no_sessions() {
//         let engine = test_engine();
//         let sessions = engine.list_sessions(10).unwrap();
//         assert!(sessions.is_empty());
//     }

//     #[test]
//     fn list_sessions_returns_created_sessions() {
//         let engine = test_engine();
//         engine.db.create_session("Session A", "model").unwrap();
//         engine.db.create_session("Session B", "model").unwrap();
//         let sessions = engine.list_sessions(10).unwrap();
//         assert_eq!(sessions.len(), 2);
//     }

//     #[test]
//     fn list_sessions_respects_limit() {
//         let engine = test_engine();
//         for i in 0..5 {
//             engine.db.create_session(&format!("S{}", i), "model").unwrap();
//         }
//         assert_eq!(engine.list_sessions(3).unwrap().len(), 3);
//     }

//     #[test]
//     fn load_session_returns_messages_in_order() {
//         let mut engine = test_engine();
//         let sid = engine.db.create_session("Test", "model").unwrap();
//         engine.db.insert_message(sid, "user", "First", None).unwrap();
//         engine.db.insert_message(sid, "assistant", "Second", None).unwrap();

//         let messages = engine.load_session(sid).unwrap();
//         assert_eq!(messages.len(), 2);
//         assert_eq!(messages[0].role, "user");
//         assert_eq!(messages[0].content, "First");
//         assert_eq!(messages[1].role, "assistant");
//         assert_eq!(messages[1].content, "Second");
//     }

//     #[test]
//     fn load_session_returns_empty_for_session_with_no_messages() {
//         let mut engine = test_engine();
//         let sid = engine.db.create_session("Empty", "model").unwrap();
//         assert!(engine.load_session(sid).unwrap().is_empty());
//     }

//     // ── delete_current_session ────────────────────────────────────────────────

//     #[test]
//     fn delete_current_session_resets_engine_state() {
//         let mut engine = test_engine();
//         let sid = engine.db.create_session("Test", "model").unwrap();
//         engine.session_id = sid;
//         engine.session_name = "Test".to_string();
//         engine.messages.push(Message::user("hi"));

//         engine.delete_current_session().unwrap();

//         assert_eq!(engine.session_id, 0);
//         assert_eq!(engine.session_name, "Untitled Session");
//         assert_eq!(engine.messages.len(), 1);
//         assert!(matches!(engine.messages[0].kind, MessageKind::Logo));
//     }

//     #[test]
//     fn delete_current_session_removes_session_from_db() {
//         let mut engine = test_engine();
//         let sid = engine.db.create_session("ToDelete", "model").unwrap();
//         engine.session_id = sid;

//         engine.delete_current_session().unwrap();

//         assert!(engine.db.get_session(sid).is_err());
//     }

//     #[test]
//     fn delete_current_session_when_no_session_does_not_error() {
//         let mut engine = test_engine();
//         // session_id is 0 — no DB row to delete
//         assert!(engine.delete_current_session().is_ok());
//     }

//     // ── end_session ───────────────────────────────────────────────────────────

//     #[test]
//     fn on_stream_complete_saves_assistant_message() {
//         let rt = tokio::runtime::Runtime::new().unwrap();
//         rt.block_on(async {
//             let mut engine = test_engine();
//             let sid = engine.db.create_session("Test", "model").unwrap();
//             engine.session_id = sid;

//             let (title_tx, _title_rx) = tokio::sync::mpsc::unbounded_channel();
//             engine.on_stream_complete("The assistant reply", title_tx).await.unwrap();

//             let messages = engine.db.get_session_messages(sid).unwrap();
//             assert_eq!(messages.len(), 1);
//             assert_eq!(messages[0].role, "assistant");
//             assert_eq!(messages[0].content, "The assistant reply");
//             assert!(!engine.assistant_state.is_active());
//         });
//     }
// }
