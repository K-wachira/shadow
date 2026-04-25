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
    Llamacpp,
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
pub struct Llamacpp {
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
                base_url: String::from("http://localhost:8080"),
                api_key: String::from("mistral"),
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

impl Default for Llamacpp {
    fn default() -> Self {
        Self {
            host: "http://127.0.0.1:8080".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_ollama_provider() {
        let config = Config::default();
        assert_eq!(config.llm_provider.provider, Backend::Ollama);
    }

    #[test]
    fn default_config_has_expected_model() {
        let config = Config::default();
        assert_eq!(config.llm_provider.model_name, "deepseek-r1:latest");
    }

    #[test]
    fn default_config_has_expected_base_url() {
        let config = Config::default();
        assert_eq!(config.llm_provider.base_url, "http://localhost:8080");
    }

    #[test]
    fn default_config_has_expected_reflection_interval() {
        let config = Config::default();
        assert_eq!(config.reflection.interval_minutes, 60);
    }

    #[test]
    fn default_config_has_expected_min_new_logs() {
        let config = Config::default();
        assert_eq!(config.reflection.min_new_logs, 5);
    }

    #[test]
    fn default_config_has_expected_api_key() {
        let config = Config::default();
        assert_eq!(config.llm_provider.api_key, "mistral");
    }

    #[test]
    fn backend_ollama_serializes_to_lowercase() {
        let serialized = serde_json::to_string(&Backend::Ollama).unwrap();
        assert_eq!(serialized, "\"ollama\"");
    }

    #[test]
    fn backend_mistral_rs_serializes_to_lowercase() {
        let serialized = serde_json::to_string(&Backend::MistralRs).unwrap();
        assert_eq!(serialized, "\"mistralrs\"");
    }

    #[test]
    fn backend_llamacpp_serializes_to_lowercase() {
        let serialized = serde_json::to_string(&Backend::Llamacpp).unwrap();
        assert_eq!(serialized, "\"llamacpp\"");
    }

    #[test]
    fn backend_deserializes_ollama() {
        let backend: Backend = serde_json::from_str("\"ollama\"").unwrap();
        assert_eq!(backend, Backend::Ollama);
    }

    #[test]
    fn backend_deserializes_mistralrs() {
        let backend: Backend = serde_json::from_str("\"mistralrs\"").unwrap();
        assert_eq!(backend, Backend::MistralRs);
    }

    #[test]
    fn backend_deserializes_llamacpp() {
        let backend: Backend = serde_json::from_str("\"llamacpp\"").unwrap();
        assert_eq!(backend, Backend::Llamacpp);
    }

    #[test]
    fn backend_unkown_deserializes() {
        let backend: Backend = serde_json::from_str("\"unknown\"").unwrap();
        assert_eq!(backend, Backend::Unknown);
    }

    #[test]
    fn backend_equality_works() {
        assert_eq!(Backend::Ollama, Backend::Ollama);
        assert_ne!(Backend::Ollama, Backend::MistralRs);
        assert_ne!(Backend::MistralRs, Backend::Llamacpp);
    }

    #[test]
    fn default_config_is_valid_toml() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.llm_provider.provider, config.llm_provider.provider);
        assert_eq!(parsed.llm_provider.model_name, config.llm_provider.model_name);
    }

    #[test]
    fn default_mistral_rs_config() {
        let config = MistralRsConfig::default();
        assert_eq!(config.host, "http://localhost:1234");
    }

    #[test]
    fn default_llamacpp_config() {
        let config = Llamacpp::default();
        assert_eq!(config.host, "http://127.0.0.1:8080");
    }
}
