//! LLM backend implementations

use super::{AgentConfig, ChatChunk, ChatMessage};
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;

/// LLM Backend trait
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Generate a response (non-streaming)
    async fn generate(&self, messages: &[ChatMessage], temperature: f32) -> Result<String>;

    /// Generate a streaming response
    async fn generate_stream(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>>;

    /// Check if backend is available
    async fn health_check(&self) -> Result<bool>;

    /// Get model info
    fn model_name(&self) -> &str;

    /// Get metadata for the most recent generation request, if available.
    fn last_generation_report(&self) -> Option<GenerationReport>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerationReport {
    pub requested_model: String,
    pub effective_model: String,
    pub requested_context_size: i32,
    pub effective_context_size: i32,
    pub model_fallback_applied: bool,
    pub context_fallback_applied: bool,
    pub continuation_attempts: usize,
}

fn build_generation_report(
    requested_model: &str,
    requested_context_size: i32,
    effective_model: &str,
    effective_context_size: i32,
    continuation_attempts: usize,
) -> GenerationReport {
    GenerationReport {
        requested_model: requested_model.to_string(),
        effective_model: effective_model.to_string(),
        requested_context_size,
        effective_context_size,
        model_fallback_applied: !requested_model.eq_ignore_ascii_case(effective_model),
        context_fallback_applied: effective_context_size != requested_context_size,
        continuation_attempts,
    }
}

#[derive(Debug, Clone)]
struct OllamaCallReport {
    effective_model: String,
    effective_context_size: i32,
}

// ============ Ollama Backend ============

/// Ollama API backend
pub struct OllamaBackend {
    client: Client,
    api_url: String,
    model: String,
    max_tokens: i32,
    context_size: i32,
    last_report: Arc<Mutex<Option<GenerationReport>>>,
}

const OLLAMA_CONTINUE_PROMPT: &str =
    "Continue from the reasoning you already completed. Do not restart or repeat the full reasoning. Provide only the final answer for the user.";
const MAX_THINKING_CONTINUATIONS: usize = 2;

impl OllamaBackend {
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            client: Client::new(),
            api_url: config.api_url.clone(),
            model: config.model_name.clone(),
            max_tokens: config.max_tokens,
            context_size: config.context_size,
            last_report: Arc::new(Mutex::new(None)),
        }
    }

    fn retry_context_sizes(&self) -> Vec<i32> {
        let mut sizes = vec![self.context_size];

        for candidate in [8192, 4096, 2048, 1024] {
            if candidate < self.context_size && !sizes.contains(&candidate) {
                sizes.push(candidate);
            }
        }

        sizes
    }

    fn fallback_model(&self) -> Option<&'static str> {
        self.model
            .eq_ignore_ascii_case("qwen3.5:9b")
            .then_some("qwen3.5:4b")
    }

    async fn send_chat_request(&self, request: &OllamaRequest) -> Result<(Response, OllamaCallReport)> {
        let mut current_request = request.clone();
        let mut allow_model_fallback = true;

        loop {
            let mut last_error = None;
            let retry_sizes = self.retry_context_sizes();
            let mut should_try_model_fallback = false;

            for (index, context_size) in retry_sizes.iter().copied().enumerate() {
                let mut request = current_request.clone();
                request.options.num_ctx = context_size;

                let response = self
                    .client
                    .post(format!("{}/api/chat", self.api_url))
                    .json(&request)
                    .send()
                    .await?;

                if response.status().is_success() {
                    if context_size != self.context_size {
                        tracing::warn!(
                            "Ollama request recovered for model '{}' after retrying with num_ctx={}",
                            current_request.model,
                            context_size
                        );
                    }

                    return Ok((
                        response,
                        OllamaCallReport {
                            effective_model: current_request.model.clone(),
                            effective_context_size: context_size,
                        },
                    ));
                }

                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                let message = if body.trim().is_empty() {
                    format!(
                        "Ollama returned {} for model '{}' with num_ctx={}",
                        status, current_request.model, context_size
                    )
                } else {
                    format!(
                        "Ollama returned {} for model '{}' with num_ctx={}: {}",
                        status,
                        current_request.model,
                        context_size,
                        body.trim()
                    )
                };

                if allow_model_fallback
                    && current_request.model.eq_ignore_ascii_case(&self.model)
                    && status.is_server_error()
                    && is_ollama_runner_resource_error(&body)
                {
                    should_try_model_fallback = true;
                }

                if status.is_server_error() && index + 1 < retry_sizes.len() {
                    tracing::warn!("{}. Retrying with a smaller context window.", message);
                    last_error = Some(anyhow::anyhow!(message));
                    continue;
                }

                last_error = Some(anyhow::anyhow!(message));
                break;
            }

            if should_try_model_fallback {
                if let Some(fallback_model) = self.fallback_model() {
                    tracing::warn!(
                        "Ollama model '{}' hit a runner/resource failure. Retrying the request with fallback model '{}'.",
                        self.model,
                        fallback_model
                    );
                    current_request.model = fallback_model.to_string();
                    allow_model_fallback = false;
                    continue;
                }
            }

            return Err(
                last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to get response from Ollama"))
            );
        }
    }
}

#[derive(Clone, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    think: bool,
    options: OllamaOptions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thinking: Option<String>,
}

#[derive(Clone, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
    num_ctx: i32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelTag>,
}

#[derive(Deserialize)]
struct OllamaModelTag {
    name: String,
}

#[derive(Deserialize)]
struct OllamaStreamChunk {
    message: Option<OllamaMessage>,
    done: bool,
}

#[derive(Debug)]
struct OllamaCompletion {
    message: OllamaMessage,
    done_reason: Option<String>,
}

impl OllamaCompletion {
    fn from_response(response: OllamaResponse) -> Self {
        Self {
            message: response.message.unwrap_or_else(|| OllamaMessage {
                role: "assistant".to_string(),
                content: String::new(),
                thinking: None,
            }),
            done_reason: response.done_reason,
        }
    }

    fn visible_content(&self) -> &str {
        self.message.content.trim()
    }

    fn is_reasoning_only_length_stop(&self) -> bool {
        self.visible_content().is_empty()
            && self
                .message
                .thinking
                .as_deref()
                .map(|thinking| !thinking.trim().is_empty())
                .unwrap_or(false)
            && matches!(self.done_reason.as_deref(), Some("length"))
    }
}

fn is_ollama_runner_resource_error(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("model runner has unexpectedly stopped")
        || lower.contains("resource limitations")
        || lower.contains("out of memory")
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    async fn generate(&self, messages: &[ChatMessage], temperature: f32) -> Result<String> {
        let mut request = OllamaRequest {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(|m| OllamaMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    thinking: None,
                })
                .collect(),
            stream: false,
            think: true,
            options: OllamaOptions {
                temperature,
                num_predict: self.max_tokens,
                num_ctx: self.context_size,
            },
        };

        for continuation_attempt in 0..=MAX_THINKING_CONTINUATIONS {
            let (response, call_report) = self.send_chat_request(&request).await?;
            let completion = OllamaCompletion::from_response(response.json().await?);

            if !completion.is_reasoning_only_length_stop() {
                if completion.visible_content().is_empty()
                    && completion
                        .message
                        .thinking
                        .as_deref()
                        .map(|thinking| !thinking.trim().is_empty())
                        .unwrap_or(false)
                {
                    tracing::warn!(
                        "Ollama model '{}' returned only reasoning without final content (done_reason={:?})",
                        self.model,
                        completion.done_reason
                    );
                }

                *self.last_report.lock().unwrap() = Some(build_generation_report(
                    &self.model,
                    self.context_size,
                    &call_report.effective_model,
                    call_report.effective_context_size,
                    continuation_attempt,
                ));

                return Ok(completion.message.content);
            }

            if continuation_attempt == MAX_THINKING_CONTINUATIONS {
                tracing::warn!(
                    "Ollama model '{}' exhausted reasoning-only continuation attempts without visible content.",
                    self.model
                );
                *self.last_report.lock().unwrap() = Some(build_generation_report(
                    &self.model,
                    self.context_size,
                    &call_report.effective_model,
                    call_report.effective_context_size,
                    continuation_attempt,
                ));
                return Ok(completion.message.content);
            }

            tracing::warn!(
                "Ollama model '{}' used the full generation budget on reasoning only; requesting continuation attempt {}.",
                self.model,
                continuation_attempt + 1
            );

            request.messages.push(completion.message);
            request.messages.push(OllamaMessage {
                role: "user".to_string(),
                content: OLLAMA_CONTINUE_PROMPT.to_string(),
                thinking: None,
            });
        }

        Ok(String::new())
    }

    async fn generate_stream(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                thinking: None,
            })
            .collect();

        let request = OllamaRequest {
            model: self.model.clone(),
            messages: ollama_messages,
            stream: true,
            think: true,
            options: OllamaOptions {
                temperature,
                num_predict: self.max_tokens,
                num_ctx: self.context_size,
            },
        };

        let (response, call_report) = self.send_chat_request(&request).await?;
        *self.last_report.lock().unwrap() = Some(build_generation_report(
            &self.model,
            self.context_size,
            &call_report.effective_model,
            call_report.effective_context_size,
            0,
        ));

        let byte_stream = response.bytes_stream();

        let stream = byte_stream.map(|result| match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(line) {
                        return Ok(ChatChunk {
                            content: chunk.message.map(|m| m.content).unwrap_or_default(),
                            done: chunk.done,
                        });
                    }
                }
                Ok(ChatChunk {
                    content: String::new(),
                    done: false,
                })
            }
            Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
        });

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.api_url))
            .send()
            .await?
            .error_for_status()?;

        match response.json::<OllamaTagsResponse>().await {
            Ok(tags) => {
                let has_model = tags
                    .models
                    .iter()
                    .any(|m| m.name.eq_ignore_ascii_case(&self.model));

                if !has_model {
                    tracing::warn!(
                        "Ollama is reachable, but model '{}' is not listed in /api/tags",
                        self.model
                    );
                }

                Ok(true)
            }
            Err(error) => {
                tracing::warn!("Failed to parse Ollama /api/tags response: {}", error);
                Ok(true)
            }
        }
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn last_generation_report(&self) -> Option<GenerationReport> {
        self.last_report.lock().unwrap().clone()
    }
}

// ============ OpenAI-Compatible Backend ============

/// OpenAI API-compatible backend
pub struct OpenAiBackend {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
    max_tokens: i32,
    context_size: i32,
    last_report: Arc<Mutex<Option<GenerationReport>>>,
}

impl OpenAiBackend {
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            client: Client::new(),
            api_url: config.api_url.clone(),
            api_key: config.api_key.clone().unwrap_or_default(),
            model: config.model_name.clone(),
            max_tokens: config.max_tokens,
            context_size: config.context_size,
            last_report: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
    delta: Option<OpenAiDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    async fn generate(&self, messages: &[ChatMessage], temperature: f32) -> Result<String> {
        let openai_messages: Vec<OpenAiMessage> = messages
            .iter()
            .map(|m| OpenAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: openai_messages,
            stream: false,
            temperature,
            max_tokens: self.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let result: OpenAiResponse = response.json().await?;

        *self.last_report.lock().unwrap() = Some(build_generation_report(
            &self.model,
            self.context_size,
            &self.model,
            self.context_size,
            0,
        ));

        Ok(result
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default())
    }

    async fn generate_stream(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let openai_messages: Vec<OpenAiMessage> = messages
            .iter()
            .map(|m| OpenAiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: openai_messages,
            stream: true,
            temperature,
            max_tokens: self.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let byte_stream = response.bytes_stream();

        *self.last_report.lock().unwrap() = Some(build_generation_report(
            &self.model,
            self.context_size,
            &self.model,
            self.context_size,
            0,
        ));

        let stream = byte_stream.map(|result| match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            return Ok(ChatChunk {
                                content: String::new(),
                                done: true,
                            });
                        }
                        if let Ok(resp) = serde_json::from_str::<OpenAiResponse>(data) {
                            let content = resp
                                .choices
                                .first()
                                .and_then(|c| c.delta.as_ref())
                                .and_then(|d| d.content.clone())
                                .unwrap_or_default();
                            let done = resp
                                .choices
                                .first()
                                .and_then(|c| c.finish_reason.as_ref())
                                .is_some();
                            return Ok(ChatChunk { content, done });
                        }
                    }
                }
                Ok(ChatChunk {
                    content: String::new(),
                    done: false,
                })
            }
            Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
        });

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/models", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        Ok(response.is_ok())
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn last_generation_report(&self) -> Option<GenerationReport> {
        self.last_report.lock().unwrap().clone()
    }
}

/// Create LLM backend from config
pub fn create_backend(config: &AgentConfig) -> Box<dyn LlmBackend> {
    match config.backend.as_str() {
        "openai" => Box::new(OpenAiBackend::new(config)),
        _ => Box::new(OllamaBackend::new(config)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_report_marks_model_and_context_fallbacks() {
        let report = build_generation_report("qwen3.5:9b", 4096, "qwen3.5:4b", 2048, 2);

        assert_eq!(report.requested_model, "qwen3.5:9b");
        assert_eq!(report.effective_model, "qwen3.5:4b");
        assert_eq!(report.requested_context_size, 4096);
        assert_eq!(report.effective_context_size, 2048);
        assert!(report.model_fallback_applied);
        assert!(report.context_fallback_applied);
        assert_eq!(report.continuation_attempts, 2);
    }

    #[test]
    fn serializes_ollama_requests_with_thinking_enabled() {
        let request = OllamaRequest {
            model: "qwen3.5:9b".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                thinking: None,
            }],
            stream: false,
            think: true,
            options: OllamaOptions {
                temperature: 0.2,
                num_predict: 128,
                num_ctx: 2048,
            },
        };

        let json = serde_json::to_value(request).expect("request should serialize");

        assert_eq!(
            json.get("think").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            json.get("messages")
                .and_then(|value| value.as_array())
                .and_then(|messages| messages.first())
                .and_then(|message| message.get("thinking")),
            None
        );
    }

    #[test]
    fn parses_thinking_and_detects_reasoning_only_length_stop() {
        let response: OllamaResponse = serde_json::from_str(
            r#"{
                "message": {
                    "role": "assistant",
                    "content": "",
                    "thinking": "long reasoning"
                },
                "done": true,
                "done_reason": "length"
            }"#,
        )
        .expect("response should deserialize");

        let completion = OllamaCompletion::from_response(response);

        assert!(completion.is_reasoning_only_length_stop());
        assert_eq!(
            completion.message.thinking.as_deref(),
            Some("long reasoning")
        );
    }

    #[test]
    fn parses_visible_content_when_reasoning_and_answer_are_present() {
        let response: OllamaResponse = serde_json::from_str(
            r#"{
                "message": {
                    "role": "assistant",
                    "content": "ready",
                    "thinking": "brief reasoning"
                },
                "done": true,
                "done_reason": "stop"
            }"#,
        )
        .expect("response should deserialize");

        let completion = OllamaCompletion::from_response(response);

        assert!(!completion.is_reasoning_only_length_stop());
        assert_eq!(completion.visible_content(), "ready");
    }

    #[test]
    fn detects_ollama_runner_resource_errors() {
        assert!(is_ollama_runner_resource_error(
            r#"{"error":"model runner has unexpectedly stopped, this may be due to resource limitations"}"#
        ));
        assert!(is_ollama_runner_resource_error("CUDA out of memory"));
        assert!(!is_ollama_runner_resource_error("some other server error"));
    }
}
