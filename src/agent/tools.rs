//! Agent tools for feed advisor

use crate::{
    db::{
        self,
        feed_labels::{display_feed_category, display_feed_classification, display_feed_name},
        feeds::Feed,
        rations::RationItem,
        Database,
    },
    diet_engine::nutrient_calc,
    scraper as feed_scraper,
};
use ::scraper::{Html, Selector};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header::CONTENT_TYPE, redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Tool trait
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool definition
    fn definition(&self) -> ToolDef;

    /// Execute the tool
    async fn execute(&self, params: Value) -> Result<String>;
}

/// Tool router manages available tools
pub struct ToolRouter {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRouter {
    /// Create new tool router
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
    }

    /// Get all tool definitions
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, params: Value) -> Result<String> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(params).await,
            None => Err(anyhow::anyhow!("Tool not found: {}", name)),
        }
    }

    /// Check if tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        let mut router = Self::new();
        router.register(Arc::new(WebSearchTool::new()));
        router
    }
}

impl ToolRouter {
    pub fn with_feed_library(feed_library: LocalFeedLibrary) -> Self {
        let mut router = Self::default();
        router.register(Arc::new(FeedLookupTool::new(feed_library.clone())));
        router.register(Arc::new(NutrientCalculatorTool::new(feed_library.clone())));
        router.register(Arc::new(SuggestFeedTool::new(feed_library)));
        router
    }
}

// ============ Web Search Tool ============

/// Web search tool using DuckDuckGo and direct page fetches.
pub struct WebSearchTool {
    client: Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .redirect(Policy::limited(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }
}

#[derive(Debug, Clone)]
struct SearchHit {
    title: String,
    url: String,
    search_snippet: String,
    page_excerpt: Option<String>,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "web_search".to_string(),
            description:
                "Search the web for information about animal nutrition, feed prices, or research"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let query = params["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return Ok("No query provided for web search.".to_string());
        }

        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?
            .error_for_status()?;

        let html = response.text().await?;
        let mut hits = {
            let document = Html::parse_document(&html);
            let result_selector = Selector::parse(".result").unwrap();
            let title_selector = Selector::parse(".result__a").unwrap();
            let snippet_selector = Selector::parse(".result__snippet").unwrap();

            let mut hits = Vec::new();
            for result in document.select(&result_selector).take(5) {
                let title_el = result.select(&title_selector).next();
                let title = title_el
                    .as_ref()
                    .map(|el| normalize_whitespace(&el.text().collect::<String>()))
                    .unwrap_or_default();
                let raw_href = title_el
                    .and_then(|el| el.value().attr("href"))
                    .unwrap_or("");
                let source_url = normalize_search_url(raw_href);
                let snippet = result
                    .select(&snippet_selector)
                    .next()
                    .map(|el| normalize_whitespace(&el.text().collect::<String>()))
                    .unwrap_or_default();

                if source_url == "N/A" {
                    continue;
                }

                if !title.is_empty() || !snippet.is_empty() {
                    hits.push(SearchHit {
                        title,
                        url: source_url,
                        search_snippet: truncate_chars(&snippet, 260),
                        page_excerpt: None,
                    });
                }
            }

            hits
        };

        for hit in hits.iter_mut().take(4) {
            match fetch_page_excerpt(&self.client, &hit.url).await {
                Ok(preview) => {
                    if hit.title.is_empty() {
                        if let Some(page_title) = preview.title {
                            hit.title = page_title;
                        }
                    }
                    hit.page_excerpt = preview.excerpt;
                }
                Err(error) => {
                    tracing::debug!("Failed to fetch page preview for {}: {}", hit.url, error);
                }
            }
        }

        let rendered: Vec<String> = hits
            .iter()
            .enumerate()
            .map(|(index, hit)| {
                let mut block = vec![format!("[{}] {}", index + 1, fallback_title(hit))];
                block.push(format!("URL: {}", hit.url));
                if !hit.search_snippet.is_empty() {
                    block.push(format!("Search snippet: {}", hit.search_snippet));
                }
                if let Some(page_excerpt) = &hit.page_excerpt {
                    if !page_excerpt.is_empty() {
                        block.push(format!("Page excerpt: {}", page_excerpt));
                    }
                }
                block.join("\n")
            })
            .collect();

        if rendered.is_empty() {
            Ok(format!("No search results found for '{}'.", query))
        } else {
            Ok(format!(
                "Web research for '{}':\n\n{}",
                query,
                rendered.join("\n\n")
            ))
        }
    }
}

#[derive(Debug, Default)]
struct PagePreview {
    title: Option<String>,
    excerpt: Option<String>,
}

async fn fetch_page_excerpt(client: &Client, url: &str) -> Result<PagePreview> {
    let response = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await?
        .error_for_status()?;

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !(content_type.contains("text/html")
        || content_type.contains("text/plain")
        || content_type.is_empty())
    {
        anyhow::bail!("Unsupported content type: {}", content_type);
    }

    let body = response.text().await?;
    if body.trim().is_empty() {
        return Ok(PagePreview::default());
    }

    if content_type.contains("text/plain") {
        return Ok(PagePreview {
            title: None,
            excerpt: Some(truncate_chars(&normalize_whitespace(&body), 420)),
        });
    }

    Ok(extract_html_preview(&body))
}

fn extract_html_preview(html: &str) -> PagePreview {
    let document = Html::parse_document(html);
    let title_selector = Selector::parse("title").unwrap();
    let meta_selector =
        Selector::parse("meta[name=\"description\"], meta[property=\"og:description\"]").unwrap();
    let paragraph_selector = Selector::parse("p").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|node| normalize_whitespace(&node.text().collect::<String>()))
        .filter(|text| !text.is_empty())
        .map(|text| truncate_chars(&text, 160));

    let description = document
        .select(&meta_selector)
        .find_map(|node| node.value().attr("content"))
        .map(normalize_whitespace)
        .filter(|text| !text.is_empty());

    let paragraph = document
        .select(&paragraph_selector)
        .map(|node| normalize_whitespace(&node.text().collect::<String>()))
        .find(|text| text.len() >= 80);

    let excerpt = match (description, paragraph) {
        (Some(description), Some(paragraph)) => {
            if paragraph
                .to_lowercase()
                .contains(&description.to_lowercase())
            {
                Some(truncate_chars(&description, 420))
            } else {
                Some(truncate_chars(
                    &format!("{} {}", description, paragraph),
                    420,
                ))
            }
        }
        (Some(description), None) => Some(truncate_chars(&description, 420)),
        (None, Some(paragraph)) => Some(truncate_chars(&paragraph, 420)),
        (None, None) => None,
    };

    PagePreview { title, excerpt }
}

fn fallback_title(hit: &SearchHit) -> String {
    if hit.title.is_empty() {
        hit.url.clone()
    } else {
        hit.title.clone()
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn normalize_search_url(raw_href: &str) -> String {
    if raw_href.is_empty() {
        return "N/A".to_string();
    }

    let candidate = if raw_href.starts_with("//") {
        format!("https:{}", raw_href)
    } else if raw_href.starts_with('/') {
        format!("https://duckduckgo.com{}", raw_href)
    } else {
        raw_href.to_string()
    };

    if let Ok(parsed) = Url::parse(&candidate) {
        if parsed
            .domain()
            .is_some_and(|domain| domain.ends_with("duckduckgo.com"))
        {
            if let Some((_, value)) = parsed.query_pairs().find(|(key, _)| key == "uddg") {
                return value.into_owned();
            }
        }
        return parsed.to_string();
    }

    candidate
}

// ============ Feed Lookup Tool ============

#[derive(Clone)]
pub struct LocalFeedLibrary {
    db: Arc<Database>,
}

impl LocalFeedLibrary {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn summary_line(&self) -> Result<String> {
        let feeds = self.all_feeds()?;
        if feeds.is_empty() {
            return Ok("Local feed library is available but currently empty.".to_string());
        }

        let mut categories = HashMap::<String, usize>::new();
        for feed in &feeds {
            *categories.entry(feed.category.clone()).or_insert(0) += 1;
        }

        let mut ranked_categories: Vec<(String, usize)> = categories.into_iter().collect();
        ranked_categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let category_summary = ranked_categories
            .into_iter()
            .take(5)
            .map(|(category, count)| format!("{} ({})", category, count))
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!(
            "Local feed library available in SQLite: {} feeds. Top categories: {}.",
            feeds.len(),
            category_summary
        ))
    }

    pub fn relevant_context_for_query(&self, query: &str, limit: usize) -> Result<Option<String>> {
        let mut matches = self.search_by_name(query, limit)?;
        if matches.is_empty() {
            if let Some(category) = resolve_category_key(query) {
                matches = self.search_by_category(&category, limit)?;
            }
        }

        if matches.is_empty() {
            return Ok(None);
        }

        Ok(Some(format_feed_table(
            "Relevant local feed matches",
            &matches,
            None,
        )))
    }

    pub fn search_by_name(&self, query: &str, limit: usize) -> Result<Vec<Feed>> {
        let feeds = self.all_feeds()?;
        let query_normalized = normalize_text(query);
        if query_normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored: Vec<(Feed, i32)> = feeds
            .into_iter()
            .filter_map(|feed| {
                let score = score_feed_match(&query_normalized, &feed);
                (score > 0).then_some((feed, score))
            })
            .collect();

        scored.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| feed_quality_score(&b.0).cmp(&feed_quality_score(&a.0)))
        });

        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(feed, _)| feed)
            .collect())
    }

    pub fn search_by_category(&self, category: &str, limit: usize) -> Result<Vec<Feed>> {
        let category = resolve_category_key(category).unwrap_or_else(|| normalize_text(category));
        self.ensure_seeded()?;
        self.db.with_conn(|conn| {
            db::feeds::list_feeds(conn, Some(&category), None, Some(limit as i64), None)
        })
    }

    pub fn filter_by_nutrient(
        &self,
        nutrient: &str,
        min_value: f64,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Feed>> {
        let nutrient_key = canonical_nutrient_key(nutrient)
            .ok_or_else(|| anyhow::anyhow!("Unsupported nutrient '{}'.", nutrient))?;
        let category_key = category.and_then(resolve_category_key);
        let feeds = self.all_feeds()?;

        let mut filtered: Vec<Feed> = feeds
            .into_iter()
            .filter(|feed| {
                let category_match = category_key
                    .as_ref()
                    .map(|key| feed.category == *key)
                    .unwrap_or(true);
                category_match && feed.nutrient_value(nutrient_key) >= min_value
            })
            .collect();

        filtered.sort_by(|a, b| {
            b.nutrient_value(nutrient_key)
                .partial_cmp(&a.nutrient_value(nutrient_key))
                .unwrap_or(Ordering::Equal)
                .then_with(|| feed_quality_score(b).cmp(&feed_quality_score(a)))
        });

        filtered.truncate(limit);
        Ok(filtered)
    }

    pub fn best_match(&self, name: &str) -> Result<Option<Feed>> {
        Ok(self.search_by_name(name, 1)?.into_iter().next())
    }

    fn all_feeds(&self) -> Result<Vec<Feed>> {
        self.ensure_seeded()?;
        self.db
            .with_conn(|conn| db::feeds::list_feeds(conn, None, None, Some(500), None))
    }

    fn ensure_seeded(&self) -> Result<()> {
        let total = self
            .db
            .with_conn(|conn| db::feeds::count_feeds(conn, None))?;
        if total == 0 {
            feed_scraper::seed_from_json_if_empty(self.db.as_ref())?;
        }
        Ok(())
    }
}

fn format_feed_table(title: &str, feeds: &[Feed], nutrient_focus: Option<&str>) -> String {
    let mut lines = vec![
        format!("### {}", title),
        String::new(),
        "| ID | Feed | Category | DM % | CP g/kg | Crude fiber g/kg | Price RUB/t |".to_string(),
        "| ---: | --- | --- | ---: | ---: | ---: | ---: |".to_string(),
    ];

    for feed in feeds {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            feed.id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            escape_markdown_cell(&display_feed_name(feed)),
            escape_markdown_cell(&display_feed_classification(feed)),
            format_optional(feed.dry_matter),
            format_optional(feed.crude_protein),
            format_optional(feed.crude_fiber),
            format_optional(feed.price_per_ton),
        ));
    }

    lines.push(String::new());
    lines.push(match nutrient_focus {
        Some(nutrient) => format!(
            "Source: local Felex feed library. Rows are ranked by `{}`.",
            nutrient
        ),
        None => "Source: local Felex feed library.".to_string(),
    });

    if feeds.len() == 1 {
        let feed = &feeds[0];
        let mut details = Vec::new();
        if let Some(value) = feed.energy_oe_cattle {
            details.push(format!("- OE cattle: **{:.2} MJ/kg DM**", value));
        }
        if let Some(value) = feed.calcium {
            details.push(format!("- Calcium: **{:.2} g/kg**", value));
        }
        if let Some(value) = feed.phosphorus {
            details.push(format!("- Phosphorus: **{:.2} g/kg**", value));
        }
        if let Some(value) = &feed.source_url {
            details.push(format!("- Source URL: {}", value));
        }
        if let Some(value) = &feed.notes {
            if !value.trim().is_empty() {
                details.push(format!("- Notes: {}", value.trim()));
            }
        }

        if !details.is_empty() {
            lines.push(String::new());
            lines.push("Additional details:".to_string());
            lines.extend(details);
        }
    }

    lines.join("\n")
}

fn format_nutrient_summary(
    title: &str,
    summary: &nutrient_calc::NutrientSummary,
    matched_feeds: &[String],
    unresolved_feeds: &[String],
) -> String {
    let mut lines = vec![
        format!("### {}", title),
        String::new(),
        "| Metric | Value |".to_string(),
        "| --- | ---: |".to_string(),
        format!("| Total weight | {:.2} kg/day |", summary.total_weight_kg),
        format!("| Dry matter | {:.2} kg/day |", summary.total_dm_kg),
        format!("| Energy (EKE) | {:.2} |", summary.energy_eke),
        format!(
            "| Energy (OE cattle) | {:.2} MJ/day |",
            summary.energy_oe_cattle
        ),
        format!("| Crude protein | {:.2} g/day |", summary.crude_protein),
        format!("| Crude fiber | {:.2} g/day |", summary.crude_fiber),
        format!("| Calcium | {:.2} g/day |", summary.calcium),
        format!("| Phosphorus | {:.2} g/day |", summary.phosphorus),
    ];

    if !matched_feeds.is_empty() {
        lines.push(String::new());
        lines.push(format!("Matched feeds: {}", matched_feeds.join(", ")));
    }

    if !unresolved_feeds.is_empty() {
        lines.push(format!(
            "Unresolved names: {}. These items were skipped because no local feed-library match was found.",
            unresolved_feeds.join(", ")
        ));
    }

    lines.push(
        "Source: calculated from feeds resolved in the local Felex feed library.".to_string(),
    );
    lines.join("\n")
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{:.2}", number))
        .unwrap_or_else(|| "-".to_string())
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn normalize_text(value: &str) -> String {
    value.trim().to_lowercase().replace('ё', "е")
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .map(normalize_text)
        .filter(|token| token.len() >= 3 && !is_stopword(token))
        .collect()
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "from"
            | "that"
            | "this"
            | "what"
            | "about"
            | "show"
            | "list"
            | "feed"
            | "feeds"
            | "ration"
            | "diet"
            | "please"
            | "цена"
            | "цены"
            | "для"
            | "или"
            | "что"
            | "как"
            | "какой"
            | "покажи"
            | "найди"
            | "список"
            | "корм"
            | "корма"
            | "рацион"
    )
}

fn feed_quality_score(feed: &Feed) -> i32 {
    let mut score = 0;
    if feed.verified {
        score += 10;
    }
    if feed.price_per_ton.is_some() {
        score += 2;
    }
    if feed.source_url.is_some() {
        score += 1;
    }
    score
}

fn score_feed_match(query: &str, feed: &Feed) -> i32 {
    let haystack = normalize_text(&format!(
        "{} {} {} {}",
        feed.name_ru,
        feed.name_en.clone().unwrap_or_default(),
        feed.category,
        feed.subcategory.clone().unwrap_or_default(),
    ));

    let mut score = 0;
    if haystack == query {
        score += 150;
    }
    if haystack.contains(query) {
        score += 80 + query.len() as i32;
    }

    let mut seen = HashSet::new();
    for token in tokenize(query) {
        if !seen.insert(token.clone()) {
            continue;
        }

        if haystack.contains(&token) {
            score += 18 + token.len() as i32;
        }

        if let Some(category) = resolve_category_key(&token) {
            if feed.category == category {
                score += 12;
            }
            if display_feed_category(&feed.category)
                .to_lowercase()
                .contains(&token)
            {
                score += 6;
            }
        }
    }

    score
}

fn resolve_category_key(value: &str) -> Option<String> {
    let normalized = normalize_text(value);
    const CATEGORY_ALIASES: &[(&str, &[&str])] = &[
        ("grain", &["grain", "grains", "зерно", "зерновые"]),
        (
            "concentrate",
            &["concentrate", "concentrates", "концентрат", "концентраты"],
        ),
        (
            "oilseed_meal",
            &["oilseed", "meal", "meals", "шрот", "жмых", "шроты", "жмыхи"],
        ),
        ("protein", &["protein", "protein_feed", "белок", "белковые"]),
        (
            "roughage",
            &["roughage", "forage", "hay", "солома", "сено", "грубые"],
        ),
        ("silage", &["silage", "haylage", "силос", "сенаж"]),
        ("succulent", &["succulent", "roots", "корнеплоды", "сочные"]),
        (
            "animal_origin",
            &["animal", "fishmeal", "животного", "животные"],
        ),
        (
            "mineral",
            &["mineral", "minerals", "минерал", "минеральные"],
        ),
        ("premix", &["premix", "premixes", "премикс", "премиксы"]),
        ("additive", &["additive", "additives", "добавка", "добавки"]),
        ("other", &["other", "прочее", "прочие"]),
    ];

    CATEGORY_ALIASES.iter().find_map(|(key, aliases)| {
        aliases
            .iter()
            .any(|alias| normalized.contains(alias))
            .then(|| (*key).to_string())
    })
}

fn canonical_nutrient_key(value: &str) -> Option<&'static str> {
    let normalized = normalize_text(value);
    match normalized.as_str() {
        "cp" | "crude_protein" | "protein" | "сырой протеин" | "протеин" | "белок" => {
            Some("crude_protein")
        }
        "fat" | "crude_fat" | "жир" | "сырой жир" => Some("crude_fat"),
        "fiber" | "fibre" | "crude_fiber" | "клетчатка" | "сырая клетчатка" => {
            Some("crude_fiber")
        }
        "calcium" | "ca" | "кальций" => Some("calcium"),
        "phosphorus" | "p" | "фосфор" => Some("phosphorus"),
        "lysine" | "лизин" => Some("lysine"),
        "methionine+cystine" | "methionine_cystine" | "метионин+цистин" | "метионин + цистин" => {
            Some("methionine_cystine")
        }
        "energy" | "oe" | "energy_oe_cattle" | "энергия" | "оэ" => {
            Some("energy_oe_cattle")
        }
        _ => None,
    }
}

/// Feed database lookup tool
pub struct FeedLookupTool {
    feed_library: LocalFeedLibrary,
}

impl FeedLookupTool {
    pub fn new(feed_library: LocalFeedLibrary) -> Self {
        Self { feed_library }
    }
}

#[async_trait]
impl Tool for FeedLookupTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "feed_lookup".to_string(),
            description: "Look up nutritional information from the local Felex feed library"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "feed_name": {
                        "type": "string",
                        "description": "Name of the feed to look up"
                    },
                    "category": {
                        "type": "string",
                        "description": "Optional feed category filter"
                    },
                    "nutrient": {
                        "type": "string",
                        "description": "Optional nutrient key such as crude_protein, crude_fiber, calcium, phosphorus"
                    },
                    "min_value": {
                        "type": "number",
                        "description": "Optional minimum nutrient value for nutrient searches"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of rows to return"
                    }
                }
            }),
        }
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let limit = params["limit"].as_u64().unwrap_or(6).clamp(1, 12) as usize;
        let feed_name = params["feed_name"].as_str().unwrap_or("").trim();
        let category = params["category"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let nutrient = params["nutrient"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let min_value = params["min_value"].as_f64().unwrap_or(0.0);

        if let Some(nutrient_name) = nutrient {
            let matches =
                self.feed_library
                    .filter_by_nutrient(nutrient_name, min_value, category, limit)?;
            if matches.is_empty() {
                return Ok(format!(
                    "No local feed-library entries matched nutrient `{}` with minimum value `{:.2}`.",
                    nutrient_name, min_value
                ));
            }
            return Ok(format_feed_table(
                &format!("Local feed lookup by nutrient `{}`", nutrient_name),
                &matches,
                canonical_nutrient_key(nutrient_name),
            ));
        }

        if let Some(category_name) = category {
            let matches = self.feed_library.search_by_category(category_name, limit)?;
            if matches.is_empty() {
                return Ok(format!(
                    "No feeds found in the local feed library for category `{}`.",
                    category_name
                ));
            }
            return Ok(format_feed_table(
                &format!("Local feed lookup for category `{}`", category_name),
                &matches,
                None,
            ));
        }

        if feed_name.is_empty() {
            return Ok(
                "Provide `feed_name`, `category`, or `nutrient` to query the local feed library."
                    .to_string(),
            );
        }

        let matches = self.feed_library.search_by_name(feed_name, limit)?;
        if matches.is_empty() {
            return Ok(format!(
                "Feed '{}' was not found in the local Felex feed library.",
                feed_name
            ));
        }

        Ok(format_feed_table(
            &format!("Local feed lookup for `{}`", feed_name),
            &matches,
            None,
        ))
    }
}

// ============ Nutrient Calculator Tool ============

/// Nutrient calculation tool
pub struct NutrientCalculatorTool {
    feed_library: LocalFeedLibrary,
}

impl NutrientCalculatorTool {
    pub fn new(feed_library: LocalFeedLibrary) -> Self {
        Self { feed_library }
    }
}

#[async_trait]
impl Tool for NutrientCalculatorTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "calculate_nutrients".to_string(),
            description: "Calculate total nutrients from a list of feeds and amounts".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "feeds": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "amount_kg": {"type": "number"}
                            }
                        },
                        "description": "List of feeds with amounts in kg"
                    }
                },
                "required": ["feeds"]
            }),
        }
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let feeds = params["feeds"].as_array();

        if feeds.is_none() || feeds.unwrap().is_empty() {
            return Ok("No feeds provided for calculation.".to_string());
        }

        let mut ration_items = Vec::new();
        let mut matched_feeds = Vec::new();
        let mut unresolved_feeds = Vec::new();

        for (index, feed) in feeds.unwrap().iter().enumerate() {
            let name = feed["name"].as_str().unwrap_or("").trim();
            let amount = feed["amount_kg"].as_f64().unwrap_or(0.0);

            if name.is_empty() || amount <= 0.0 {
                continue;
            }

            match self.feed_library.best_match(name)? {
                Some(feed_match) => {
                    matched_feeds.push(display_feed_name(&feed_match));
                    ration_items.push(RationItem {
                        id: None,
                        ration_id: 0,
                        feed_id: feed_match.id.unwrap_or_default(),
                        feed: Some(feed_match),
                        amount_kg: amount,
                        is_locked: false,
                        sort_order: index as i32,
                    });
                }
                None => unresolved_feeds.push(name.to_string()),
            }
        }

        if ration_items.is_empty() {
            return Ok(
                "No feed names could be resolved against the local Felex feed library.".to_string(),
            );
        }

        let summary = nutrient_calc::calculate_nutrients(&ration_items);
        Ok(format_nutrient_summary(
            "Calculated nutrients from local feed-library feeds",
            &summary,
            &matched_feeds,
            &unresolved_feeds,
        ))
    }
}

// ============ Feed Suggestion Tool ============

pub struct SuggestFeedTool {
    feed_library: LocalFeedLibrary,
}

impl SuggestFeedTool {
    pub fn new(feed_library: LocalFeedLibrary) -> Self {
        Self { feed_library }
    }
}

#[async_trait]
impl Tool for SuggestFeedTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "suggest_feed".to_string(),
            description: "Suggest feeds from the local feed library ranked by nutrient content"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "nutrient": {
                        "type": "string",
                        "description": "Nutrient to rank by, e.g. crude_protein, lysine, calcium"
                    },
                    "deficiency_amount": {
                        "type": "number",
                        "description": "Optional deficiency amount used only for narrative context"
                    },
                    "category": {
                        "type": "string",
                        "description": "Optional category filter"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of suggestions"
                    }
                },
                "required": ["nutrient"]
            }),
        }
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let nutrient = params["nutrient"].as_str().unwrap_or("").trim();
        if nutrient.is_empty() {
            return Ok("No nutrient provided for feed suggestions.".to_string());
        }

        let category = params["category"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = params["limit"].as_u64().unwrap_or(5).clamp(1, 10) as usize;
        let deficiency_amount = params["deficiency_amount"].as_f64();

        let matches = self
            .feed_library
            .filter_by_nutrient(nutrient, 0.0, category, limit)?;

        if matches.is_empty() {
            return Ok(format!(
                "No local feed-library suggestions are available for nutrient `{}`.",
                nutrient
            ));
        }

        let mut response = format_feed_table(
            &format!("Suggested feeds for `{}`", nutrient),
            &matches,
            canonical_nutrient_key(nutrient),
        );

        if let Some(amount) = deficiency_amount {
            response.push_str(&format!(
                "\n\nDeficiency context: target shortfall was approximately **{:.2}** units of `{}`.",
                amount, nutrient
            ));
        }

        Ok(response)
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
                ' ' => result.push('+'),
                _ => {
                    for b in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
}
