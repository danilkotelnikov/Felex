//! Agent manager - orchestrates LLM, tools, and retrieval

use super::{
    config::AgentConfig,
    embeddings::EmbeddingModel,
    llm::{create_backend, GenerationReport, LlmBackend},
    prompt::{create_context_prompt, format_tool_result, parse_tool_call, tool_instructions},
    retriever::FeedRetriever,
    tools::{LocalFeedLibrary, ToolRouter},
    AgentStatus, ChatChunk, ChatMessage,
};
use crate::db::Database;
use anyhow::Result;
use futures::Stream;
use serde_json::json;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

/// Agent manager
pub struct AgentManager {
    config: AgentConfig,
    llm: Box<dyn LlmBackend>,
    tools: ToolRouter,
    feed_library: LocalFeedLibrary,
    retriever: Option<FeedRetriever>,
    status: Arc<RwLock<AgentStatus>>,
}

impl AgentManager {
    /// Create new agent manager.
    /// Automatically starts Ollama if needed and waits for it to become available.
    pub async fn new(config: AgentConfig, db: Arc<Database>) -> Result<Self> {
        if config.backend == "ollama" {
            ensure_ollama_running(&config.api_url).await;
        }

        let llm = create_backend(&config);
        let model_loaded = check_with_retries(&*llm, 5, 2000).await;

        let status = AgentStatus {
            model_loaded,
            model_name: config.model_name.clone(),
            backend: config.backend.clone(),
            web_enabled: config.web_enabled,
            context_size: config.context_size,
            embedding_model: config.embedding_model.clone(),
        };

        if model_loaded {
            tracing::info!(
                "Agent connected to {} via {}",
                config.model_name,
                config.backend
            );
        } else {
            tracing::warn!(
                "Agent could not connect to {} at {} - will retry on next request",
                config.model_name,
                config.api_url
            );
        }

        let feed_library = LocalFeedLibrary::new(db);

        let mut manager = Self {
            config,
            llm,
            tools: ToolRouter::with_feed_library(feed_library.clone()),
            feed_library,
            retriever: None,
            status: Arc::new(RwLock::new(status)),
        };

        if let Some(ref embed_model) = manager.config.embedding_model {
            let embedding = EmbeddingModel::new(&manager.config.api_url, embed_model);
            let mut retriever = FeedRetriever::new(embedding);

            if retriever.initialize().await.is_ok() {
                manager.retriever = Some(retriever);
            }
        }

        Ok(manager)
    }

    /// Get current status
    pub fn status(&self) -> AgentStatus {
        self.status.read().unwrap().clone()
    }

    pub fn last_generation_report(&self) -> Option<GenerationReport> {
        self.llm.last_generation_report()
    }

    /// Update status after health check
    pub async fn refresh_status(&self) {
        let model_loaded = self.llm.health_check().await.unwrap_or(false);
        let mut status = self.status.write().unwrap();
        status.model_loaded = model_loaded;
        status.model_name = self.config.model_name.clone();
        status.backend = self.config.backend.clone();
        status.web_enabled = self.config.web_enabled;
        status.context_size = self.config.context_size;
    }

    /// Chat with the agent (non-streaming)
    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        context: Option<ChatContext>,
    ) -> Result<String> {
        self.ensure_model_ready().await?;

        if let Some(response) = self
            .try_web_grounded_answer(messages, context.clone())
            .await?
        {
            return Ok(response);
        }

        let mut full_messages = self
            .build_messages(messages, context, true, Vec::new())
            .await?;

        let mut raw_response = self
            .llm
            .generate(&full_messages, self.config.temperature)
            .await?;
        let mut response = clean_model_response(&raw_response);

        tracing::debug!("Agent raw response length: {} chars", raw_response.len());

        let mut tool_iterations = 0;
        while tool_iterations < 5 {
            let Some((tool_name, params)) = parse_tool_call(&raw_response) else {
                break;
            };

            if !self.tools.has_tool(&tool_name) {
                break;
            }

            tracing::info!("Agent executing tool: {}", tool_name);

            let tool_result = if tool_name == "web_search" && !self.config.web_enabled {
                "Web search is disabled in settings. Use the local feed library and current ration context only.".to_string()
            } else {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(20),
                    self.tools.execute(&tool_name, params),
                )
                .await
                {
                    Ok(Ok(result)) => result,
                    Ok(Err(e)) => format!("Tool error: {}", e),
                    Err(_) => "Tool timed out after 20 seconds".to_string(),
                }
            };

            full_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: raw_response.clone(),
            });
            full_messages.push(ChatMessage {
                role: "user".to_string(),
                content: format_tool_result(&tool_name, &tool_result),
            });

            raw_response = self
                .llm
                .generate(&full_messages, self.config.temperature)
                .await?;
            response = clean_model_response(&raw_response);
            tool_iterations += 1;
        }

        if response.is_empty() {
            response = self
                .force_finalize(&mut full_messages, &raw_response, false)
                .await?;
        }

        if response.is_empty() {
            anyhow::bail!("Не удалось сформировать итоговый ответ. Повторите запрос.");
        }

        Ok(response)
    }

    /// Chat with streaming response
    pub async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        context: Option<ChatContext>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        self.ensure_model_ready().await?;

        let full_messages = self
            .build_messages(messages, context, true, Vec::new())
            .await?;

        self.llm
            .generate_stream(&full_messages, self.config.temperature)
            .await
    }

    async fn ensure_model_ready(&self) -> Result<()> {
        let needs_recheck = !self.status.read().unwrap().model_loaded;
        if needs_recheck {
            let ok = self.llm.health_check().await.unwrap_or(false);
            if ok {
                self.status.write().unwrap().model_loaded = true;
            } else {
                anyhow::bail!(format!(
                    "Модель {} недоступна. Проверьте, что Ollama запущен и модель установлена.",
                    self.config.model_name
                ));
            }
        }

        Ok(())
    }

    async fn try_web_grounded_answer(
        &self,
        messages: &[ChatMessage],
        _context: Option<ChatContext>,
    ) -> Result<Option<String>> {
        if !self.config.web_enabled {
            return Ok(None);
        }

        let Some(last_user) = messages.iter().rev().find(|message| message.role == "user") else {
            return Ok(None);
        };

        if !should_prefetch_web(&last_user.content) {
            return Ok(None);
        }

        let Some(web_context) = self.prefetch_web_context(&last_user.content).await else {
            return Ok(None);
        };

        let mut full_messages = self.build_web_research_messages(messages, &web_context);

        let raw_response = self
            .llm
            .generate(&full_messages, self.config.temperature)
            .await?;
        let mut response = clean_model_response(&raw_response);

        if response.is_empty() || !contains_sources_section(&response) {
            response = self
                .force_finalize(&mut full_messages, &raw_response, true)
                .await?;
        }

        response = ensure_sources_section(response, &web_context);

        if response.is_empty() || looks_like_generic_intake(&response) {
            return Ok(Some(build_research_summary(
                &last_user.content,
                &web_context,
            )));
        }

        Ok(Some(response))
    }

    async fn force_finalize(
        &self,
        full_messages: &mut Vec<ChatMessage>,
        raw_response: &str,
        require_sources: bool,
    ) -> Result<String> {
        if !raw_response.trim().is_empty() {
            full_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: raw_response.to_string(),
            });
        }

        let instruction = if require_sources {
            "Now provide the final answer in Markdown using the web research already supplied. Summarize the findings, answer directly in the user's language, and finish with a `URLs:` section that contains only raw source URLs, one per line. Do not ask clarifying questions. Do not call tools or output XML tags."
        } else {
            "Now provide the final answer in Markdown using the tool results already supplied. Do not call tools or output XML tags."
        };

        full_messages.push(ChatMessage {
            role: "user".to_string(),
            content: instruction.to_string(),
        });

        let finalized = self
            .llm
            .generate(full_messages, self.config.temperature)
            .await?;

        Ok(clean_model_response(&finalized))
    }

    fn build_web_research_messages(
        &self,
        messages: &[ChatMessage],
        web_context: &str,
    ) -> Vec<ChatMessage> {
        let mut full_messages = Vec::new();
        full_messages.push(ChatMessage {
            role: "system".to_string(),
            content: "You are Felex in web research mode. Answer only the user's actual question. Use the supplied web research package, synthesize across sources, and reply in the user's language using Markdown. Do not switch into generic ration-planning guidance unless the user explicitly asked for ration formulation. Do not ask clarifying questions if the web package already contains enough information. End every answer with a `URLs:` section containing only raw source URLs, one per line.".to_string(),
        });
        full_messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!("Web research package:\n{}", web_context),
        });
        full_messages.extend(messages.iter().cloned());
        full_messages
    }

    /// Build full message list with system prompt and context
    async fn build_messages(
        &self,
        messages: &[ChatMessage],
        context: Option<ChatContext>,
        include_tools: bool,
        extra_system_messages: Vec<String>,
    ) -> Result<Vec<ChatMessage>> {
        let mut full_messages = Vec::new();

        let system_prompt = if let Some(ctx) = context {
            create_context_prompt(
                &ctx.animal_type,
                &ctx.production_level,
                ctx.current_ration.as_deref(),
                ctx.nutrient_status.as_deref(),
            )
        } else {
            super::prompt::SYSTEM_PROMPT.to_string()
        };

        let system_prompt = if include_tools {
            format!(
                "{}\n{}",
                system_prompt,
                tool_instructions(self.config.web_enabled)
            )
        } else {
            system_prompt
        };

        full_messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        self.append_retrieval_context(&mut full_messages, messages)
            .await;
        self.append_local_feed_context(&mut full_messages, messages);

        for message in extra_system_messages {
            if !message.trim().is_empty() {
                full_messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: message,
                });
            }
        }

        full_messages.extend(messages.iter().cloned());

        Ok(full_messages)
    }

    async fn append_retrieval_context(
        &self,
        full_messages: &mut Vec<ChatMessage>,
        messages: &[ChatMessage],
    ) {
        if let Some(ref retriever) = self.retriever {
            if let Some(last_user) = messages.iter().rev().find(|message| message.role == "user") {
                if let Ok(relevant) = retriever.retrieve(&last_user.content, 3).await {
                    if !relevant.is_empty() {
                        full_messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: format!("Relevant knowledge:\n{}", relevant.join("\n\n")),
                        });
                    }
                }
            }
        }
    }

    fn append_local_feed_context(
        &self,
        full_messages: &mut Vec<ChatMessage>,
        messages: &[ChatMessage],
    ) {
        let mut blocks = Vec::new();

        if let Ok(summary) = self.feed_library.summary_line() {
            if !summary.trim().is_empty() {
                blocks.push(summary);
            }
        }

        if let Some(last_user) = messages.iter().rev().find(|message| message.role == "user") {
            if let Ok(Some(context)) = self
                .feed_library
                .relevant_context_for_query(&last_user.content, 5)
            {
                blocks.push(context);
            }
        }

        if !blocks.is_empty() {
            full_messages.push(ChatMessage {
                role: "system".to_string(),
                content: format!("Local feed library context:\n{}", blocks.join("\n\n")),
            });
        }
    }

    async fn prefetch_web_context(&self, query: &str) -> Option<String> {
        match tokio::time::timeout(
            std::time::Duration::from_secs(18),
            self.tools
                .execute("web_search", json!({ "query": query.to_string() })),
        )
        .await
        {
            Ok(Ok(result)) if result.trim().is_empty() => None,
            Ok(Ok(result)) if result.contains("No search results") => None,
            Ok(Ok(result)) => Some(result),
            Ok(Err(error)) => {
                tracing::debug!("Web prefetch failed: {}", error);
                None
            }
            Err(_) => {
                tracing::debug!("Web prefetch timed out");
                None
            }
        }
    }
}

// ============ Response Cleaning ============

fn clean_model_response(text: &str) -> String {
    strip_tool_markup(&strip_thinking(text))
}

/// Strip `<think>...</think>` blocks from thinking-model responses (Qwen3.5, etc.)
fn strip_thinking(text: &str) -> String {
    let re = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap();
    let cleaned = re.replace_all(text, "");
    cleaned.trim().to_string()
}

fn strip_tool_markup(text: &str) -> String {
    let tool_re = regex::Regex::new(r"(?is)<tool>[\s\S]*?</tool>").unwrap();
    let params_re = regex::Regex::new(r"(?is)<params>[\s\S]*?</params>").unwrap();
    let result_re = regex::Regex::new(r"(?is)<tool_result[^>]*>[\s\S]*?</tool_result>").unwrap();

    let without_tool = tool_re.replace_all(text, "");
    let without_params = params_re.replace_all(&without_tool, "");
    let without_results = result_re.replace_all(&without_params, "");
    without_results.trim().to_string()
}

fn should_prefetch_web(query: &str) -> bool {
    let query = query.to_lowercase();
    let web_keywords = [
        "latest",
        "current",
        "today",
        "news",
        "update",
        "updates",
        "research",
        "study",
        "studies",
        "market",
        "price",
        "prices",
        "cost",
        "forecast",
        "recent",
        "source",
        "sources",
        "internet",
        "web",
        "find",
        "search",
        "2025",
        "2026",
        "сегодня",
        "новости",
        "послед",
        "цена",
        "цены",
        "стоим",
        "сколько стоит",
        "рын",
        "актуал",
        "источник",
        "источники",
        "интернет",
        "веб",
        "найди",
        "найти",
        "поиск",
        "исслед",
    ];

    web_keywords.iter().any(|keyword| query.contains(keyword))
}

fn contains_sources_section(text: &str) -> bool {
    regex::Regex::new(r"(?im)^(sources|источники|urls)\s*:")
        .unwrap()
        .is_match(text)
}

fn ensure_sources_section(answer: String, web_context: &str) -> String {
    if answer.trim().is_empty() {
        return answer;
    }

    let sources = extract_sources(web_context);
    if sources.is_empty() {
        return answer;
    }

    if regex::Regex::new(r"https?://").unwrap().is_match(&answer) {
        return answer;
    }

    format!("{}\n\nURLs:\n{}", answer.trim(), sources.join("\n"))
}

fn extract_sources(web_context: &str) -> Vec<String> {
    let url_re = regex::Regex::new(r"^URL:\s+(.+)$").unwrap();
    let mut sources = Vec::new();

    for line in web_context.lines() {
        let trimmed = line.trim();
        if let Some(captures) = url_re.captures(trimmed) {
            let url = captures
                .get(1)
                .map(|m| m.as_str().trim())
                .unwrap_or_default();
            if !url.is_empty() && !sources.iter().any(|existing| existing == url) {
                sources.push(url.to_string());
            }
        }
    }

    sources
}

#[derive(Default)]
struct ResearchItem {
    title: String,
    url: String,
    search_snippet: String,
    page_excerpt: String,
}

fn parse_research_items(web_context: &str) -> Vec<ResearchItem> {
    let title_re = regex::Regex::new(r"^\[(\d+)\]\s+(.+)$").unwrap();
    let mut items = Vec::new();
    let mut current: Option<ResearchItem> = None;

    for line in web_context.lines() {
        let trimmed = line.trim();

        if let Some(captures) = title_re.captures(trimmed) {
            if let Some(item) = current.take() {
                if !item.url.is_empty() {
                    items.push(item);
                }
            }
            current = Some(ResearchItem {
                title: captures
                    .get(2)
                    .map(|m| m.as_str())
                    .unwrap_or_default()
                    .to_string(),
                ..Default::default()
            });
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("URL: ") {
            if let Some(item) = current.as_mut() {
                item.url = value.trim().to_string();
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("Search snippet: ") {
            if let Some(item) = current.as_mut() {
                item.search_snippet = value.trim().to_string();
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("Page excerpt: ") {
            if let Some(item) = current.as_mut() {
                item.page_excerpt = value.trim().to_string();
            }
        }
    }

    if let Some(item) = current {
        if !item.url.is_empty() {
            items.push(item);
        }
    }

    items
}

fn build_research_summary(query: &str, web_context: &str) -> String {
    let items = parse_research_items(web_context);
    if items.is_empty() {
        return build_research_digest(web_context);
    }

    let mut lines = vec![
        format!("Краткая сводка по запросу: {}", query.trim()),
        "Собраны несколько актуальных web-источников. Формулировки и цены могут отличаться по дате публикации, региону и базису поставки.".to_string(),
        String::new(),
    ];

    for (index, item) in items.iter().take(4).enumerate() {
        let detail = if !item.page_excerpt.is_empty() {
            truncate_summary_text(&item.page_excerpt, 260)
        } else if !item.search_snippet.is_empty() {
            truncate_summary_text(&item.search_snippet, 220)
        } else {
            "См. материал по URL ниже.".to_string()
        };

        lines.push(format!("- [{}] {}: {}", index + 1, item.title, detail));
    }

    lines.push(String::new());
    lines.push("URLs:".to_string());
    for url in extract_sources(web_context) {
        lines.push(url);
    }

    lines.join("\n")
}

fn truncate_summary_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn looks_like_generic_intake(text: &str) -> bool {
    let lower = text.to_lowercase();
    let markers = [
        "пожалуйста, уточните",
        "какие животные",
        "что у вас есть в наличии",
        "какой уровень продуктивности",
        "хотите также узнать текущие цены",
        "please clarify",
        "what animals",
        "what do you have available",
    ];

    markers
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count()
        >= 1
}

fn build_research_digest(web_context: &str) -> String {
    let sources = extract_sources(web_context);
    if sources.is_empty() {
        return format!(
            "Не удалось синтезировать web-ответ моделью, но web-данные были получены:\n\n{}",
            web_context.trim()
        );
    }

    format!(
        "Не удалось синтезировать полноценный web-ответ моделью. URLs:\n{}",
        sources.join("\n")
    )
}

// ============ Ollama Auto-Launch ============

/// Ensure Ollama is running. If not, start `ollama serve` in the background.
async fn ensure_ollama_running(api_url: &str) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    if client
        .get(format!("{}/api/tags", api_url))
        .send()
        .await
        .is_ok()
    {
        tracing::info!("Ollama is already running at {}", api_url);
        return;
    }

    tracing::info!("Ollama is not running, attempting to auto-start...");

    #[cfg(target_os = "windows")]
    let launch_result = {
        let ollama_paths = [
            "ollama",
            r"C:\Users\danil\AppData\Local\Programs\Ollama\ollama.exe",
        ];

        let mut launched = false;
        for path in &ollama_paths {
            match std::process::Command::new(path)
                .arg("serve")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_child) => {
                    tracing::info!("Launched Ollama via: {} serve", path);
                    launched = true;
                    break;
                }
                Err(e) => {
                    tracing::debug!("Failed to launch {} serve: {}", path, e);
                }
            }
        }
        launched
    };

    #[cfg(not(target_os = "windows"))]
    let launch_result = {
        match std::process::Command::new("ollama")
            .arg("serve")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => {
                tracing::info!("Launched ollama serve");
                true
            }
            Err(e) => {
                tracing::warn!("Failed to launch ollama serve: {}", e);
                false
            }
        }
    };

    if !launch_result {
        tracing::error!("Could not start Ollama. Please install it from https://ollama.com");
        return;
    }

    for i in 1..=15 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if client
            .get(format!("{}/api/tags", api_url))
            .send()
            .await
            .is_ok()
        {
            tracing::info!("Ollama started successfully after {} seconds", i);
            return;
        }
        tracing::debug!("Waiting for Ollama to start... ({}/15)", i);
    }

    tracing::warn!("Ollama was launched but did not respond within 15 seconds");
}

/// Health check with retries
async fn check_with_retries(llm: &dyn LlmBackend, retries: u32, delay_ms: u64) -> bool {
    for attempt in 0..retries {
        if llm.health_check().await.unwrap_or(false) {
            return true;
        }
        if attempt < retries - 1 {
            tracing::debug!(
                "Health check attempt {} failed, retrying in {}ms...",
                attempt + 1,
                delay_ms
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }
    false
}

/// Context for chat
#[derive(Debug, Clone)]
pub struct ChatContext {
    pub animal_type: String,
    pub production_level: String,
    pub current_ration: Option<String>,
    pub nutrient_status: Option<String>,
}

impl ChatContext {
    pub fn dairy_cow(milk_kg: f64) -> Self {
        Self {
            animal_type: "Dairy Cow".to_string(),
            production_level: format!("{} kg milk/day", milk_kg),
            current_ration: None,
            nutrient_status: None,
        }
    }

    pub fn with_ration(mut self, ration: &str) -> Self {
        self.current_ration = Some(ration.to_string());
        self
    }

    pub fn with_nutrients(mut self, status: &str) -> Self {
        self.nutrient_status = Some(status.to_string());
        self
    }
}
