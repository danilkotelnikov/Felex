//! Agent configuration

use serde::{Deserialize, Serialize};

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// LLM backend type: "ollama", "openai", "llama_cpp"
    pub backend: String,

    /// Model name (e.g., "qwen3.5:4b", "gpt-4o-mini")
    pub model_name: String,

    /// API endpoint for the backend
    pub api_url: String,

    /// API key (for OpenAI-compatible backends)
    pub api_key: Option<String>,

    /// Temperature for generation
    pub temperature: f32,

    /// Max tokens to generate
    pub max_tokens: i32,

    /// Context window size (512-131072 tokens)
    pub context_size: i32,

    /// Enable web search tools
    pub web_enabled: bool,

    /// Enable RAG/embedding features (experimental)
    pub rag_enabled: bool,

    /// Embedding model (for RAG)
    pub embedding_model: Option<String>,

    /// Path to vector database
    pub vector_db_path: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            backend: "ollama".to_string(),
            model_name: "qwen3.5:4b".to_string(),
            api_url: "http://localhost:11434".to_string(),
            api_key: None,
            temperature: 0.7,
            max_tokens: 2048,
            context_size: 16384,
            web_enabled: true,
            rag_enabled: false,
            embedding_model: Some("nomic-embed-text".to_string()),
            vector_db_path: Some("data/vectors.db".to_string()),
        }
    }
}

impl AgentConfig {
    /// Create config for Ollama backend
    pub fn ollama(model: &str) -> Self {
        Self {
            backend: "ollama".to_string(),
            model_name: model.to_string(),
            api_url: "http://localhost:11434".to_string(),
            ..Default::default()
        }
    }

    /// Create config for OpenAI-compatible backend
    pub fn openai(model: &str, api_key: &str, api_url: Option<&str>) -> Self {
        Self {
            backend: "openai".to_string(),
            model_name: model.to_string(),
            api_url: api_url.unwrap_or("https://api.openai.com/v1").to_string(),
            api_key: Some(api_key.to_string()),
            ..Default::default()
        }
    }

    /// Load from environment or config file
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(backend) = std::env::var("FELEX_LLM_BACKEND") {
            config.backend = backend;
        }
        if let Ok(model) = std::env::var("FELEX_LLM_MODEL") {
            config.model_name = model;
        }
        if let Ok(url) = std::env::var("FELEX_LLM_URL") {
            config.api_url = url;
        }
        if let Ok(key) = std::env::var("FELEX_LLM_API_KEY") {
            config.api_key = Some(key);
        }
        if let Ok(context_size) = std::env::var("FELEX_CONTEXT_SIZE") {
            config.context_size =
                normalize_context_size(context_size.parse().unwrap_or(config.context_size));
        }
        if let Ok(web) = std::env::var("FELEX_WEB_ENABLED") {
            config.web_enabled = web.parse().unwrap_or(true);
        }

        config
    }
}

fn normalize_context_size(value: i32) -> i32 {
    value.clamp(512, 131072)
}
