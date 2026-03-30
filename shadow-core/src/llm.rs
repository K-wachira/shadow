use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use anyhow::Result;
use mistralrs::{Model};
use std::pin::Pin;
use futures::Stream;
use tokio_stream::StreamExt;

pub type LlmStream = Pin<Box<dyn Stream<Item = String> + Send>>;

pub enum LlmProvider {
    Ollama(Ollama),
    MistralRs(Model),
}

pub struct LlmClient {
    pub provider: LlmProvider,
    pub model: String,
}

#[allow(dead_code)]
impl LlmClient {
    pub fn init(provider: LlmProvider, model: &str) -> Result<Self, String> {
        Ok(Self {
            provider: provider,
            model: model.to_string(),
        })
    }
    
    
    pub async fn llm_ask(&self, prompt: &str) -> color_eyre::Result<String> {
        match &self.provider {
            LlmProvider::Ollama(ollama) => {
                self.ollama_ask(ollama, self.model.clone(), prompt).await
            }
            LlmProvider::MistralRs(_) => unimplemented!(),
        }
    }
    
    pub async fn ollama_ask(&self, ollama: &Ollama, model: String, prompt: &str) -> color_eyre::Result<String>{
        let res = ollama.generate(GenerationRequest::new(model, prompt)).await
                .map_err(|e| color_eyre::eyre::eyre!(e))?;
        Ok(res.response)
    }
    
    pub async fn llm_ask_stream(&self, prompt: &str) -> color_eyre::Result<LlmStream> {
        match &self.provider {
            LlmProvider::Ollama(ollama) => {
                self.ollama_ask_stream(ollama, self.model.clone(), prompt).await
            }
            LlmProvider::MistralRs(_) => unimplemented!(),
        }
    }
    
    pub async fn ollama_ask_stream(
        &self,
        ollama: &Ollama,
        model: String,
        prompt: &str,
    ) -> color_eyre::Result<LlmStream> {
        
        let stream = ollama
            .generate_stream(GenerationRequest::new(model, prompt.to_string()))
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
    
        let mapped = stream.map(|chunk| {
            match chunk {
                Ok(responses) => responses
                    .into_iter()
                    .map(|r| r.response)
                    .collect::<Vec<String>>()
                    .join(""),
                Err(_) => String::new(),
            }
        });
    
        Ok(Box::pin(mapped))
    }
}    
