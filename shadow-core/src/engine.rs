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
use crate::model::MessageKind;
use tokio::sync::mpsc;

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
            session_name: String::from("Untitled Session"),
            model: model.to_string(),
            assistant_state: AssistantState::Idle,
            messages: vec![],
        })
    }
    
    pub fn start_session(&mut self) -> color_eyre::Result<()> {
        let session_id = self.db.create_session(&self.session_name, &self.model)?;
        self.session_id = session_id;
        self.messages.push(Message::logo());
        Ok(())
    }
    
    pub async fn send_message(&mut self, prompt: &str) -> color_eyre::Result<impl Stream<Item = String> + 'static> {
        // create session on first message
        if self.session_id == 0 {
            self.start_session()?;
        }
       
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
    
    pub async fn on_stream_complete(&mut self, response: &str, title_tx: mpsc::UnboundedSender<String>) -> color_eyre::Result<()> {
        self.db.insert_message(
            self.session_id,
            "assistant",
            response,
            Some(&self.model),
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
    
    fn spawn_title_generation(&mut self, title_tx: mpsc::UnboundedSender<String>)  {
        eprintln!("on_stream_complete called, session_name: {}", self.session_name);

        let ollama = Arc::clone(&self.ollama);

        // grab first user + assistant exchange
        let context: String = self.messages.iter()
            .filter_map(|m| match &m.kind {
                MessageKind::UserInput { text } => Some(format!("User: {}", text)),
                MessageKind::AssistantText { text } => Some(format!("Assistant: {}", text)),
                _ => None,
            })
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");
    
        tokio::spawn(async move {
            let prompt = format!(
                "Generate a short session title (5 words max). Return only the title as plain text, nothing else.\n\n{}",
                context
            );
            match ollama.ollama_ask(&prompt).await {
                Ok(title) => {
                    eprintln!("Generated title: {}", title);
                    let _ = title_tx.send(title.trim().to_string());
                }
                Err(e) => {
                    eprintln!("Title generation failed: {}", e);
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
        self.messages.push(Message::logo());
        Ok(())
    }
}