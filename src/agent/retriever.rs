//! RAG Retriever for feed knowledge

use super::embeddings::{EmbeddingModel, VectorStore};
use anyhow::Result;
use serde_json::json;

/// Feed knowledge retriever
pub struct FeedRetriever {
    vector_store: VectorStore,
    initialized: bool,
}

impl FeedRetriever {
    /// Create new retriever
    pub fn new(embedding_model: EmbeddingModel) -> Self {
        Self {
            vector_store: VectorStore::new(embedding_model),
            initialized: false,
        }
    }

    /// Initialize with feed knowledge base
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Add feed categories knowledge
        let knowledge_base = vec![
            ("feed_categories", "Feed categories include: concentrates (grains, oilseed meals), roughage (hay, straw), silage (corn silage, haylage), minerals (limestone, phosphates), and premixes (vitamin-mineral supplements)."),
            ("energy_feeds", "Energy-rich feeds include corn grain (14.2 MJ/kg DM), barley (12.8 MJ/kg DM), wheat (13.5 MJ/kg DM), and molasses. These are high in starch and digestible carbohydrates."),
            ("protein_feeds", "Protein feeds include soybean meal (44% CP), sunflower meal (38% CP), canola meal (38% CP), and distillers grains. Essential amino acids lysine and methionine+cystine remain core monogastric protein indicators."),
            ("fiber_sources", "Fiber sources provide structural carbohydrate and crude fiber support for ration design. Alfalfa hay, grass hay, and wheat straw remain common roughage sources."),
            ("dairy_nutrition", "Dairy cows producing 30-35 kg milk/day require strong energy supply, adequate crude protein, balanced calcium and phosphorus, and controlled starch exposure."),
            ("mineral_balance", "Calcium and phosphorus balance is critical. Dairy cows need 120-150g Ca and 80-100g P daily. Calcium sources: limestone (38% Ca). Phosphorus sources: MCP (23% P)."),
            ("amino_acids", "Limiting amino-acid control in the current Felex surface focuses on lysine and methionine+cystine for swine and poultry."),
            ("fiber_management", "When only crude-fiber and starch data are available, ration interpretation should separate structural-fiber risk from concentrate-driven starch pressure."),
            ("vitamin_requirements", "The implemented vitamin surface tracks vitamin A, vitamin D3, vitamin E, carotene, and selenium directly from the feed database."),
            ("cost_optimization", "Feed cost optimization considers: price per unit energy (RUB/MJ), price per unit protein (RUB/g CP), and minimum cost formulation using linear programming."),
            ("silage_quality", "Quality corn silage: 30-35% DM, 10-11 MJ OE/kg DM, >26% starch. Fermentation pH <4.0. Poor fermentation reduces intake and milk production."),
        ];

        for (id, content) in knowledge_base {
            self.vector_store
                .add(id, content, json!({"type": "knowledge"}))
                .await?;
        }

        self.initialized = true;
        Ok(())
    }

    /// Retrieve relevant knowledge for a query
    pub async fn retrieve(&self, query: &str, top_k: usize) -> Result<Vec<String>> {
        let results = self.vector_store.search(query, top_k).await?;
        Ok(results.into_iter().map(|(doc, _)| doc.content).collect())
    }

    /// Add custom knowledge
    pub async fn add_knowledge(&mut self, id: &str, content: &str) -> Result<()> {
        self.vector_store
            .add(id, content, json!({"type": "custom"}))
            .await
    }

    /// Check if initialized
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}
