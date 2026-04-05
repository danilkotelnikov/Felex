//! Embeddings generation for RAG

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Embedding model interface
pub struct EmbeddingModel {
    client: Client,
    api_url: String,
    model: String,
}

impl EmbeddingModel {
    /// Create new embedding model (uses Ollama)
    pub fn new(api_url: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            model: model.to_string(),
        }
    }

    /// Create with default Ollama settings
    pub fn ollama_default() -> Self {
        Self::new("http://localhost:11434", "nomic-embed-text")
    }

    /// Generate embedding for a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/embeddings", self.api_url))
            .json(&request)
            .send()
            .await?;

        let result: EmbedResponse = response.json().await?;
        Ok(result.embedding)
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

/// Simple in-memory vector store
pub struct VectorStore {
    documents: Vec<Document>,
    embedding_model: EmbeddingModel,
}

/// Document with embedding
#[derive(Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Vec<f32>,
}

impl VectorStore {
    /// Create new vector store
    pub fn new(embedding_model: EmbeddingModel) -> Self {
        Self {
            documents: Vec::new(),
            embedding_model,
        }
    }

    /// Add document to store
    pub async fn add(
        &mut self,
        id: &str,
        content: &str,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let embedding = self.embedding_model.embed(content).await?;
        self.documents.push(Document {
            id: id.to_string(),
            content: content.to_string(),
            metadata,
            embedding,
        });
        Ok(())
    }

    /// Search for similar documents
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<(Document, f32)>> {
        let query_embedding = self.embedding_model.embed(query).await?;

        let mut results: Vec<(Document, f32)> = self
            .documents
            .iter()
            .map(|doc| {
                let score = EmbeddingModel::cosine_similarity(&query_embedding, &doc.embedding);
                (doc.clone(), score)
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top_k
        Ok(results.into_iter().take(top_k).collect())
    }

    /// Get document count
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}
