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
    
    async fn ollama_ask(&self, prompt: &String) -> Result<String, OllamaError>{
        let model = "gemma3:12b".to_string();
        let res = self.llm.generate(GenerationRequest::new(model, prompt)).await;
        
        match res {
            Ok(result) => Ok(result.response),
            Err(err) =>Err(err)   
        }
    }
    
    pub async fn ollama_ask_stream(
        &self, 
        prompt: &String
    ) -> Result<impl Stream<Item = Result<Vec<GenerationResponse>, OllamaError>>, OllamaError> {
        let model = "gemma3:12b".to_string();
        
        // The library returns a Result<LlamaStream, OllamaError>
        // LlamaStream implements Stream<Item = Result<Vec<GenerationResponse>, OllamaError>>
        let stream = self.llm
            .generate_stream(GenerationRequest::new(model, prompt.clone()))
            .await?;
    
        Ok(stream)
    }
}    
