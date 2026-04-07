use reqwest::Client;
use futures::Stream;
use futures::StreamExt;
use serde::Serialize;
use serde::Deserialize;
use std::pin::Pin;
use crate::config::Config;

pub type LlmStream = Pin<Box<dyn Stream<Item = String> + Send>>;

pub struct LlmClient {
    pub client: Client,
    pub base_url: String,
    pub model_name: String,
    pub api_key: String,
}

#[derive(Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: Option<ChatMessageResponse>,
    delta: Option<ChatMessageResponse>,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

impl LlmClient {
    pub fn init(config: &Config) -> color_eyre::Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: normalize_base_url(&config.llm_provider.base_url),
            model_name: config.llm_provider.model_name.clone(),
            api_key: config.llm_provider.api_key.clone(),
        })
    }
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    fn request_builder(&self) -> reqwest::RequestBuilder {
        let builder = self.client.post(self.chat_url());
        let key = if self.api_key.is_empty() || self.api_key == "None" {
            "none".to_string()
        } else {
            self.api_key.clone()
        };
        builder.header("Authorization", format!("Bearer {}", key))
    }

    pub async fn llm_ask(&self, messages: &[ChatMessage]) -> color_eyre::Result<String> {
        let response = self
            .request_builder()
            .json(&ChatRequest { model: &self.model_name, messages, stream: false })
            .send()
            .await?
            .error_for_status()?
            .json::<ChatResponse>()
            .await?;

        Ok(response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.clone())
            .unwrap_or_default())
    }

    pub async fn llm_ask_stream(&self, messages: &[ChatMessage]) -> color_eyre::Result<LlmStream> {
        tracing::debug!(">>> sending to {}", self.chat_url());
        let mut byte_stream = self
            .request_builder()
            .json(&ChatRequest { model: &self.model_name, messages, stream: true })
            .send()
            .await?
            .error_for_status()?
            .bytes_stream();
        
        tracing::debug!(">>> sent to {}", self.chat_url());
        let stream = async_stream::stream! {
            let mut pending = String::new();
            let mut seen_visible = false;

            while let Some(Ok(chunk)) = byte_stream.next().await {
                pending.push_str(&String::from_utf8_lossy(&chunk));
                pending = pending.replace("\r\n", "\n").replace('\r', "\n");

                while let Some(boundary) = pending.find("\n\n") {
                    let event = pending[..boundary].to_string();
                    pending.drain(..boundary + 2);

                    let data = event
                        .lines()
                        .filter_map(|l| l.strip_prefix("data:"))
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join("\n");

                    if data.is_empty() || data == "[DONE]" { continue; }

                    if let Ok(chunk) = serde_json::from_str::<ChatResponse>(&data) {
                        if let Some(content) = chunk.choices.first()
                            .and_then(|c| c.delta.as_ref())
                            .and_then(|d| d.content.clone())
                        {
                            if content.is_empty() { continue; }
                            if !seen_visible && content.trim().is_empty() { continue; }
                            if content.chars().any(|c| !c.is_whitespace()) {
                                seen_visible = true;
                            }
                            yield content;
                        }
                    }
                }
            }
        };
        Ok(Box::pin(stream))
    }
}

fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}