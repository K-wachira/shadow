use futures::Stream;
use mistralrs::ChatCompletionChunkResponse;
use mistralrs::ChunkChoice;
use mistralrs::Delta;
use mistralrs::GgufModelBuilder;
use mistralrs::Response;
use mistralrs::TextMessageRole;
use mistralrs::TextMessages;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::StreamExt;

pub type LlmStream = Pin<Box<dyn Stream<Item = String> + Send>>;

pub enum LlmProvider {
    Ollama(Ollama),
    MistralRs(Arc<mistralrs::Model>),
}

pub struct LlmClient {
    pub provider: LlmProvider,
    pub model_name: String,
}

impl LlmClient {
    pub async fn init(provider: &str, model: &str) -> color_eyre::Result<Self> {
        let provider = Self::build_provider(provider).await?;
        Ok(Self {
            provider: provider,
            model_name: model.to_string(),
        })
    }

    async fn build_provider(provider: &str) -> color_eyre::Result<LlmProvider> {
        match provider {
            "ollama" => {
                let ollama = Ollama::new("http://localhost".to_string(), 11434);
                Ok(LlmProvider::Ollama(ollama))
            }
            "mistralrs" => {
                let model = GgufModelBuilder::new(
                    "bartowski/Qwen2.5-3B-Instruct-GGUF",
                    vec!["Qwen2.5-3B-Instruct-Q4_K_M.gguf".to_string()],
                )
                .with_logging()
                .build()
                .await
                .map_err(|e| color_eyre::eyre::eyre!(e))?;
                Ok(LlmProvider::MistralRs(Arc::new(model)))
            }
            _ => Err(color_eyre::eyre::eyre!("Unknown provider: {}", provider)),
        }
    }

    pub async fn llm_ask(&self, prompt: &str) -> color_eyre::Result<String> {
        match &self.provider {
            LlmProvider::Ollama(model) => {
                self.ollama_ask(model, self.model_name.clone(), prompt)
                    .await
            }
            LlmProvider::MistralRs(model) => self.mistralrs_ask(Arc::clone(model), prompt).await,
        }
    }

    pub async fn llm_ask_stream(&self, prompt: &str) -> color_eyre::Result<LlmStream> {
        match &self.provider {
            LlmProvider::Ollama(ollama) => {
                self.ollama_ask_stream(ollama, self.model_name.clone(), prompt)
                    .await
            }
            LlmProvider::MistralRs(model) => {
                self.mistralrs_ask_stream(Arc::clone(model), prompt).await
            }
        }
    }

    pub async fn mistralrs_ask(
        &self, model: Arc<mistralrs::Model>, prompt: &str,
    ) -> color_eyre::Result<String> {
        let messages = TextMessages::new().add_message(TextMessageRole::User, prompt);
        let res = model
            .send_chat_request(messages)
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;

        let content = res
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .cloned()
            .unwrap_or_default();
        Ok(content)
    }

    pub async fn mistralrs_ask_stream(
        &self, model: Arc<mistralrs::Model>, prompt: &str,
    ) -> color_eyre::Result<LlmStream> {
        let messages = TextMessages::new()
            .add_message(TextMessageRole::User, prompt)
            .enable_thinking(true);

        let mapped = async_stream::stream! {
            let model = model; // move Arc into stream block
            if let Ok(mut stream) = model.stream_chat_request(messages).await {
                while let Some(chunk) = stream.next().await {
                    if let Response::Chunk(ChatCompletionChunkResponse { choices, .. }) = chunk {
                        if let Some(ChunkChoice {
                            delta: Delta { content: Some(content), .. }, ..
                        }) = choices.first() {
                            yield content.clone();
                        }
                    }
                }
            }
        };
        Ok(Box::pin(mapped))
    }

    pub async fn ollama_ask(
        &self, ollama: &Ollama, model: String, prompt: &str,
    ) -> color_eyre::Result<String> {
        let res = ollama
            .generate(GenerationRequest::new(model, prompt))
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
        Ok(res.response)
    }

    pub async fn ollama_ask_stream(
        &self, ollama: &Ollama, model: String, prompt: &str,
    ) -> color_eyre::Result<LlmStream> {
        let stream = ollama
            .generate_stream(GenerationRequest::new(model, prompt.to_string()))
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;

        let mapped = stream.map(|chunk| match chunk {
            Ok(responses) => responses
                .into_iter()
                .map(|r| r.response)
                .collect::<Vec<String>>()
                .join(""),
            Err(_) => String::new(),
        });

        Ok(Box::pin(mapped))
    }
}
