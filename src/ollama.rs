use ollama_rs::Ollama;
use ollama_rs::error::OllamaError;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::completion::GenerationResponse;
use tokio_stream::Stream;

pub struct LlmClient {
    llm: Ollama,
}

impl LlmClient {
    pub fn init() -> Result<Self, String> {
        let ollama = Ollama::new("http://localhost".to_string(), 11434);
        let llm_conn = LlmClient{ llm: ollama };
        Ok(llm_conn)
    }
    

}    
