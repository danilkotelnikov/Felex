//! AI Agent module
//!
//! Provides RAG-based feed advisor with support for multiple LLM backends:
//! - Ollama (local, recommended)
//! - OpenAI-compatible APIs
//! - Direct llama.cpp via subprocess

pub mod config;
pub mod embeddings;
pub mod llm;
pub mod manager;
pub mod prompt;
pub mod retriever;
pub mod tools;
pub mod filter;

use serde::{Deserialize, Serialize};

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub model_loaded: bool,
    pub model_name: String,
    pub backend: String,
    pub web_enabled: bool,
    pub context_size: i32,
    pub embedding_model: Option<String>,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self {
            model_loaded: false,
            model_name: "Not loaded".to_string(),
            backend: "none".to_string(),
            web_enabled: false,
            context_size: 8192,
            embedding_model: None,
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

/// Chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

/// Chat response chunk (for streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub content: String,
    pub done: bool,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: String,
    pub success: bool,
}

// Re-exports
pub use config::AgentConfig;
pub use llm::{GenerationReport, LlmBackend, OllamaBackend, OpenAiBackend};
pub use manager::AgentManager;
pub use tools::{Tool, ToolRouter};
