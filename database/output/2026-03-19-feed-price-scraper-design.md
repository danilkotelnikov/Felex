# Feed Price Scraper Design

## 1. Overview and Architecture
- **Goal:** Autonomously fetch, filter, and cache real-time market prices for agricultural feeds across RU, BY, and KZ markets.
- **Language & Stack:** Rust natively embedded within the existing Axum backend. This keeps the Tauri app as a single distributable executable without requiring a Python runtime.
- **Database:** A new SQLite table `feed_prices` to cache historical and current prices.
  - Columns: `feed_id`, `price`, `currency`, `source_url`, `confidence_score`, `timestamp`, `locale` (e.g., RU, BY, KZ).
- **Libraries:** `reqwest` for HTTP requests to marketplaces, `scraper` for HTML parsing (or direct JSON deserialization if APIs are found), and `serde_json` for communicating with the local Ollama/Qwen instance.

## 2. Scraping Data Flow (Just-In-Time)
- **Trigger:** When a user opens a feed detail view or adds a feed to a ration, the frontend makes an asynchronous request to `GET /api/feeds/{id}/price?locale={loc}`.
- **Cache Check:** The Rust backend checks `feed_prices`. If a price exists and is less than 7 days old (configurable), it returns it immediately.
- **Scraping Phase (Cache Miss):**
  1. The backend constructs a search query using the exact `name_ru` (e.g., "Молоко коровье цельное").
  2. It sends a targeted search request to 1-2 pre-configured sources (e.g., Agroserver.ru, Flagma.by, or Wildberries).
  3. It extracts the top 3 to 5 listing titles, prices, and URLs.
- **AI Filtering Phase:**
  1. To prevent overloading the computer, the system constructs a single, very concise JSON prompt containing the 3-5 extracted listings and the target feed name.
  2. It sends this payload to the local Ollama/Qwen instance (via `http://localhost:11434/api/generate`).
  3. The AI is asked to output a simple JSON array of scores (1-10) for how accurately each listing matches the target feed (filtering out retail pet food or irrelevant bulk items).
- **Aggregation:**
  1. The backend filters out any listings scoring below 7.
  2. It calculates the median price of the remaining valid listings.
  3. It handles currency conversion (e.g., BYN to RUB) using a static or cached exchange rate.
- **Response:** The final median price, source link, and confidence score are saved to SQLite and returned to the frontend to update the UI.

## 3. Frontend Integration
- The UI displays a skeleton loader or a "Syncing Price..." indicator next to the price field while the JIT process runs.
- If the local AI is unavailable or the network fails, it falls back to the last known cached price or displays "Price Unavailable".