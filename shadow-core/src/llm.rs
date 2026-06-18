use crate::config::Config;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use shadow_tools::ChatTool;
use shadow_tools::ChatToolCall;
use shadow_tools::ToolDefinition;
use shadow_tools::ToolRegistry;
use shadow_utils::utils::model_name_format;
use std::pin::Pin;

pub type LlmStream = Pin<Box<dyn Stream<Item = String> + Send>>;

#[derive(Clone)]
pub struct LlmClient {
    pub client: Client,
    pub base_url: String,
    pub model_name: String,
    pub api_key: String,
    pub tool_registry: ToolRegistry,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ChatToolCall>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            ..Self::default()
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            ..Self::default()
        }
    }

    fn from_response(message: ChatMessageResponse) -> Self {
        Self {
            role: "assistant".into(),
            content: message.content.unwrap_or_default(),
            name: None,
            tool_name: None,
            tool_call_id: None,
            tool_calls: message.tool_calls,
        }
    }

    fn tool_result(call: &ChatToolCall, content: String) -> Self {
        Self {
            role: "tool".into(),
            content,
            name: None,
            tool_name: Some(call.function.name.clone()),
            tool_call_id: call.id.clone(),
            tool_calls: Vec::new(),
        }
    }
}

impl ChatMessageResponse {
    fn empty() -> Self {
        Self {
            content: Some(String::new()),
            tool_calls: Vec::new(),
        }
    }

    fn push_content(&mut self, chunk: &str) {
        self.content.get_or_insert_with(String::new).push_str(chunk);
    }
}

#[allow(dead_code)]
fn format_tool_output(output: &str) -> String {
    const MAX_CHARS: usize = 400;
    let mut truncated = output.chars().take(MAX_CHARS).collect::<String>();
    if output.chars().count() > MAX_CHARS {
        truncated.push_str("...");
    }
    truncated
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [ChatTool]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: u32,
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: Option<ChatMessageResponse>,
    delta: Option<ChatMessageResponse>,
}

#[derive(Deserialize, Clone, Debug)]
struct ChatMessageResponse {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatToolCall>,
}

/// Maximum agentic tool-call rounds before the streamed tool loop gives up.
const MAX_TOOL_ROUNDS: usize = 8;

/// Tracks whether any visible (non-whitespace) content has streamed yet, so the
/// leading blank padding some models emit before real output can be dropped.
#[derive(Default)]
struct VisibleContent {
    seen_visible: bool,
}

impl VisibleContent {
    /// Returns the content to stream, or `None` if it should be suppressed.
    fn filter<'a>(&mut self, content: &'a str) -> Option<&'a str> {
        if content.is_empty() || (!self.seen_visible && content.trim().is_empty()) {
            return None;
        }
        if content.chars().any(|c| !c.is_whitespace()) {
            self.seen_visible = true;
        }
        Some(content)
    }
}

/// Parse an OpenAI-style SSE response into decoded chat chunks: split on blank
/// lines, strip the `data:` prefix, drop `[DONE]` and keep-alive events, and
/// yield each `ChatResponse` that decodes. Undecodable events are logged and
/// skipped; a transport error ends the stream.
fn sse_chat_chunks(response: reqwest::Response) -> impl Stream<Item = ChatResponse> + Send {
    async_stream::stream! {
        let mut byte_stream = response.bytes_stream();
        let mut pending = String::new();

        while let Some(next) = byte_stream.next().await {
            let chunk = match next {
                Ok(chunk) => chunk,
                Err(err) => {
                    tracing::error!("stream chunk failed: {}", err);
                    break;
                }
            };

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

                if data.is_empty() || data == "[DONE]" {
                    continue;
                }

                match serde_json::from_str::<ChatResponse>(&data) {
                    Ok(chunk) => yield chunk,
                    Err(err) => tracing::debug!("ignoring non-chat stream event: {}", err),
                }
            }
        }
    }
}

impl LlmClient {
    pub fn init(config: &Config) -> color_eyre::Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: normalize_base_url(&config.llm_provider.base_url),
            model_name: model_name_format(config.llm_provider.model_name.clone()),
            api_key: config.llm_provider.api_key.clone(),
            tool_registry: ToolRegistry::with_defaults(),
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

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tool_registry.register(tool);
    }

    pub async fn llm_ask(&self, messages: &[ChatMessage]) -> color_eyre::Result<String> {
        let response = self
            .request_builder()
            .json(&ChatRequest {
                model: &self.model_name,
                messages,
                stream: false,
                tools: None,
                tool_choice: None,
            })
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
        let llm = self.clone();
        let initial_history = messages.to_vec();
        let stream = async_stream::stream! {
            let tools = llm.tool_registry.schemas();
            let mut history = initial_history;

            for round in 0..MAX_TOOL_ROUNDS {
                tracing::debug!(">>> sending streamed tool turn {} to {}", round + 1, llm.chat_url());
                let send_result = llm
                    .request_builder()
                    .json(&ChatRequest {
                        model: &llm.model_name,
                        messages: &history,
                        stream: true,
                        tools: Some(&tools),
                        tool_choice: Some("auto"),
                    })
                    .send()
                    .await;

                let response = match send_result.and_then(|r| r.error_for_status()) {
                    Ok(response) => response,
                    Err(err) => {
                        tracing::error!("streamed tool request failed: {}", err);
                        break;
                    }
                };

                let mut chunks = Box::pin(sse_chat_chunks(response));
                let mut visible = VisibleContent::default();
                let mut assistant_message = ChatMessageResponse::empty();

                while let Some(chunk) = chunks.next().await {
                    // Streaming servers emit `delta`; some send a full `message`.
                    let Some(part) = chunk
                        .choices
                        .first()
                        .and_then(|c| c.delta.as_ref().or(c.message.as_ref()))
                    else {
                        continue;
                    };

                    if let Some(content) = part.content.as_deref() {
                        assistant_message.push_content(content);
                        if let Some(text) = visible.filter(content) {
                            yield text.to_string();
                        }
                    }
                    if !part.tool_calls.is_empty() {
                        assistant_message.tool_calls.extend(part.tool_calls.clone());
                    }
                }

                if assistant_message.content.as_deref().unwrap_or_default().is_empty()
                    && assistant_message.tool_calls.is_empty()
                {
                    tracing::error!("stream ended without assistant content or tool calls");
                    break;
                }
                tracing::info!(">>>> assistant_message <<<<!!! {:?} ", assistant_message);
                if assistant_message.tool_calls.is_empty() {
                    break;
                }

                history.push(ChatMessage::from_response(assistant_message.clone()));

                for call in &assistant_message.tool_calls {
                    let tool_output = match llm.tool_registry.execute(call).await {
                        Ok(output) => output,
                        Err(err) => format!("Tool error: {err}"),
                    };
                    history.push(ChatMessage::tool_result(call, tool_output));
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
