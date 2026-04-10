use crate::ask::ask;
use crate::config::Config;
use crate::db::Database;
use crate::db::SessionMessages;
use crate::db::Sessions;
use crate::llm::ChatMessage;
use crate::llm::LlmClient;
use crate::locus::mind_op;
use crate::model::AssistantState;
use crate::model::Message;
use crate::model::MessageKind;
use crate::setup::ShadowPaths;
use shadow_continuity::mind;
use shadow_continuity::mind::ShadowMind;
use shadow_services::models::EntryLog;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

pub struct Locus {
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

impl Locus {
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
        mind_op::ingest_icloud_logs(&self.db, &self.config.ingest.source_path)
    }

    pub fn list_logs(&self, limit: Option<i32>) -> color_eyre::Result<Vec<EntryLog>> {
        self.db.get_logs(limit)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::db::Database;
//     use crate::model::MessageKind;
//     use ollama_rs::Ollama;
//     use crate::llm::LlmProvider;

//     fn test_locus() -> Locus {
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
//         Locus::new(db, llm_client, "test-model").expect("locus init")
//     }

//     // ── Construction ──────────────────────────────────────────────────────────

//     #[test]
//     fn new_locus_has_no_active_session() {
//         let locus = test_locus();
//         assert_eq!(locus.session_id, 0);
//         assert_eq!(locus.session_name, "Untitled Session");
//         assert_eq!(locus.model, "test-model");
//         assert!(!locus.assistant_state.is_active());
//     }

//     #[test]
//     fn new_locus_messages_contain_logo() {
//         let locus = test_locus();
//         assert_eq!(locus.messages.len(), 1);
//         assert!(matches!(locus.messages[0].kind, MessageKind::Logo));
//     }

//     // ── start_new_session ─────────────────────────────────────────────────────

//     #[test]
//     fn start_new_session_resets_state() {
//         let mut locus = test_locus();
//         // Manually set some state to verify it gets reset
//         locus.session_id = 42;
//         locus.session_name = "Old Name".to_string();
//         locus.messages.push(Message::user("hello"));
//         locus.assistant_state = AssistantState::Thinking { secs: 5 };

//         locus.start_new_session();

//         assert_eq!(locus.session_id, 0);
//         assert_eq!(locus.session_name, "Untitled Session");
//         assert_eq!(locus.messages.len(), 1);
//         assert!(matches!(locus.messages[0].kind, MessageKind::Logo));
//         assert!(!locus.assistant_state.is_active());
//     }

//     // ── list_sessions / load_session ──────────────────────────────────────────

//     #[test]
//     fn list_sessions_empty_when_no_sessions() {
//         let locus = test_locus();
//         let sessions = locus.list_sessions(10).unwrap();
//         assert!(sessions.is_empty());
//     }

//     #[test]
//     fn list_sessions_returns_created_sessions() {
//         let locus = test_locus();
//         locus.db.create_session("Session A", "model").unwrap();
//         locus.db.create_session("Session B", "model").unwrap();
//         let sessions = locus.list_sessions(10).unwrap();
//         assert_eq!(sessions.len(), 2);
//     }

//     #[test]
//     fn list_sessions_respects_limit() {
//         let locus = test_locus();
//         for i in 0..5 {
//             locus.db.create_session(&format!("S{}", i), "model").unwrap();
//         }
//         assert_eq!(locus.list_sessions(3).unwrap().len(), 3);
//     }

//     #[test]
//     fn load_session_returns_messages_in_order() {
//         let mut locus = test_locus();
//         let sid = locus.db.create_session("Test", "model").unwrap();
//         locus.db.insert_message(sid, "user", "First", None).unwrap();
//         locus.db.insert_message(sid, "assistant", "Second", None).unwrap();

//         let messages = locus.load_session(sid).unwrap();
//         assert_eq!(messages.len(), 2);
//         assert_eq!(messages[0].role, "user");
//         assert_eq!(messages[0].content, "First");
//         assert_eq!(messages[1].role, "assistant");
//         assert_eq!(messages[1].content, "Second");
//     }

//     #[test]
//     fn load_session_returns_empty_for_session_with_no_messages() {
//         let mut locus = test_locus();
//         let sid = locus.db.create_session("Empty", "model").unwrap();
//         assert!(locus.load_session(sid).unwrap().is_empty());
//     }

//     // ── delete_current_session ────────────────────────────────────────────────

//     #[test]
//     fn delete_current_session_resets_locus_state() {
//         let mut locus = test_locus();
//         let sid = locus.db.create_session("Test", "model").unwrap();
//         locus.session_id = sid;
//         locus.session_name = "Test".to_string();
//         locus.messages.push(Message::user("hi"));

//         locus.delete_current_session().unwrap();

//         assert_eq!(locus.session_id, 0);
//         assert_eq!(locus.session_name, "Untitled Session");
//         assert_eq!(locus.messages.len(), 1);
//         assert!(matches!(locus.messages[0].kind, MessageKind::Logo));
//     }

//     #[test]
//     fn delete_current_session_removes_session_from_db() {
//         let mut locus = test_locus();
//         let sid = locus.db.create_session("ToDelete", "model").unwrap();
//         locus.session_id = sid;

//         locus.delete_current_session().unwrap();

//         assert!(locus.db.get_session(sid).is_err());
//     }

//     #[test]
//     fn delete_current_session_when_no_session_does_not_error() {
//         let mut locus = test_locus();
//         // session_id is 0 — no DB row to delete
//         assert!(locus.delete_current_session().is_ok());
//     }

//     // ── end_session ───────────────────────────────────────────────────────────

//     #[test]
//     fn on_stream_complete_saves_assistant_message() {
//         let rt = tokio::runtime::Runtime::new().unwrap();
//         rt.block_on(async {
//             let mut locus = test_locus();
//             let sid = locus.db.create_session("Test", "model").unwrap();
//             locus.session_id = sid;

//             let (title_tx, _title_rx) = tokio::sync::mpsc::unbounded_channel();
//             locus.on_stream_complete("The assistant reply", title_tx).await.unwrap();

//             let messages = locus.db.get_session_messages(sid).unwrap();
//             assert_eq!(messages.len(), 1);
//             assert_eq!(messages[0].role, "assistant");
//             assert_eq!(messages[0].content, "The assistant reply");
//             assert!(!locus.assistant_state.is_active());
//         });
//     }
// }
