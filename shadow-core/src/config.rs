// config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub llm_provider: CoreLLM,
    pub reflection: ReflectionConfig,
    pub ingest: IngestConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CoreLLM {
    pub provider: Backend,
    pub model_name: String, 
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Ollama,
    MistralRs,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OllamaConfig {
    pub host: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MistralRsConfig {
    pub host: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReflectionConfig {
    pub interval_minutes: u64,
    pub min_new_logs: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IngestConfig {
    pub source_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm_provider: CoreLLM {
                provider: Backend::Ollama,
                model_name: "deepseek-r1:latest".to_string(),
                base_url : String::from("http://localhost:8080"),
                api_key : String::from("mistral")
            },
            reflection: ReflectionConfig {
                interval_minutes: 60,
                min_new_logs: 5,
            },
            ingest: IngestConfig {
                source_path: PathBuf::from(
                    "~/Library/Mobile Documents/com~apple~CloudDocs/ShadowLogs/",
                ),
            },
        }
    }
}

impl Default for MistralRsConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost:1234".to_string(),
        }
    }
}
