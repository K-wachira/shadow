use std::sync::Arc;
use crate::db::Database;
use crate::ollama::LlmClient;
use crate::model::Message;
use crate::model::AssistantState;
use tokio_stream::Stream;
use crate::ask::ask;
use tokio_stream::StreamExt;
use crate::db::Sessions;
use crate::db::SessionMessages;

pub struct ShadowEngine {
    pub db: Arc<Database>,
    pub ollama: Arc<LlmClient>,
    pub session_id: i64,
    pub session_name: String,
    pub model: String,
    pub assistant_state: AssistantState,
    pub messages: Vec<Message>,
}

impl ShadowEngine {
    pub fn new(db: Arc<Database>, ollama: Arc<LlmClient>, model: &str) -> color_eyre::Result<Self> {
        Ok(Self {
            db,
            ollama,
            session_id: 0,
            session_name: String::new(),
            model: model.to_string(),
            assistant_state: AssistantState::Idle,
            messages: vec![],
        })
    }
    
    pub fn start_session(&mut self, name: &str) -> color_eyre::Result<()> {
        let session_id = self.db.create_session(name, &self.model)?;
        self.session_id = session_id;
        self.session_name = name.to_string();
        self.messages.push(Message::logo());
        Ok(())
    }
    
    pub async fn send_message(&mut self, prompt: &str) -> color_eyre::Result<impl Stream<Item = String> + 'static> {
        self.messages.push(Message::user(prompt));
        self.db.insert_message(self.session_id, "user", prompt, None)?;
        self.assistant_state = AssistantState::Thinking { secs: 0 };
    
        let enriched = ask(&prompt.to_string(), &self.db, &self.messages).map_err(|e| color_eyre::eyre::eyre!(e))?;
        let ollama = Arc::clone(&self.ollama);
    
        let stream = async_stream::stream! {
            if let Ok(mut s) = ollama.ollama_ask_stream(&enriched).await {
                while let Some(chunk) = s.next().await {
                    if let Ok(res) = chunk {
                        for r in res {
                            yield r.response;
                        }
                    }
                }
            }
        };
    
        Ok(stream)
    }
    
    pub fn on_stream_complete(&mut self, response: &str) -> color_eyre::Result<()> {
        self.db.insert_message(
            self.session_id,
            "assistant",
            response,
            Some(&self.model),
        )?;
        self.assistant_state = AssistantState::Idle;
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
}