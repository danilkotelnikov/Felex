use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::scraper::marketplace::Listing;
use std::time::Duration;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub async fn score_listings(feed_name: &str, listings: &[Listing]) -> Result<Vec<i32>> {
    // If no listings, return empty
    if listings.is_empty() {
        return Ok(vec![]);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(5)) // Fast timeout so we fallback quickly
        .build()?;

    let mut prompt = format!(
        "Rate the relevance of each listing to the agricultural feed '{}'. \
        Output ONLY a JSON array of integers from 1 to 10. No other text.\n\nListings:\n",
        feed_name
    );

    for (i, listing) in listings.iter().enumerate() {
        prompt.push_str(&format!("{}. {}\n", i + 1, listing.title));
    }

    let req = OllamaRequest {
        model: "qwen2.5:0.5b".to_string(), // Typical small model in Felex, but any will do
        prompt,
        stream: false,
    };

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&req)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            if let Ok(ollama_resp) = response.json::<OllamaResponse>().await {
                // Try to parse the output as a JSON array
                let text = ollama_resp.response.trim();
                let text = if let Some(start) = text.find('[') {
                    if let Some(end) = text.rfind(']') {
                        &text[start..=end]
                    } else { text }
                } else { text };

                if let Ok(scores) = serde_json::from_str::<Vec<i32>>(text) {
                    if scores.len() == listings.len() {
                        return Ok(scores);
                    }
                }
            }
        }
        _ => {} // Fallback triggered below
    }

    // Fallback: If Ollama is not installed, unreachable, or returns garbage,
    // assume all scraped listings are perfectly relevant (10)
    tracing::warn!("Ollama unavailable or failed to parse scores. Using fallback scoring for feed prices.");
    Ok(vec![10; listings.len()])
}
