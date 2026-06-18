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
    pub context_tokens: i64,
    pub ephemeral: Option<String>,
}

impl Locus {
    pub fn new(
        db: Arc<Database>, llm_client: Arc<LlmClient>, config: Config, paths: ShadowPaths,
    ) -> color_eyre::Result<Self> {
        let mind = mind::load(&paths.mind)?;
        let model_name_temp = llm_client.model_name.clone();
        Ok(Self {
            db,
            llm_client,
            session_id: 0,
            session_name: String::from("Untitled Session"),
            assistant_state: AssistantState::Idle,
            messages: vec![Message::logo(model_name_temp)],
            mind,
            config,
            paths,
            context_tokens: 0,
            ephemeral: None,
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
        self.context_tokens = 0;
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
            match llm_client.llm_ask_stream(&enriched).await {
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
