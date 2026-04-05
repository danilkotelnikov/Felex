# Feed Price Scraper Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a Just-In-Time (JIT) web scraper in the Rust backend that fetches, AI-filters, and caches real-time market prices for agricultural feeds.

**Architecture:** A new Axum route will handle price requests. It will first check a SQLite cache. On a miss, it uses `reqwest` and `scraper` to fetch listings from a marketplace, sends those listings to a local Ollama instance for relevance scoring, calculates the median price of high-scoring listings, caches the result, and returns it to the frontend.

**Tech Stack:** Rust, Axum, SQLite (sqlx or rusqlite), `reqwest`, `scraper`, `serde_json`, React (Frontend).

---

### Task 1: Database Setup for Price Caching

**Files:**
- Modify: `src/db/schema.sql` (or equivalent migration file)
- Modify: `src/db/models.rs`
- Test: `tests/db_prices_test.rs`

- [ ] **Step 1: Write the failing test for DB schema**
Write a test in `tests/db_prices_test.rs` that attempts to insert and retrieve a record from the `feed_prices` table.
- [ ] **Step 2: Run test to verify it fails**
Run the test. Expected: FAIL (table does not exist).
- [ ] **Step 3: Write minimal implementation**
Add the `feed_prices` table definition to the database schema/migrations:
```sql
CREATE TABLE IF NOT EXISTS feed_prices (
    feed_id INTEGER NOT NULL,
    price REAL NOT NULL,
    currency TEXT NOT NULL,
    source_url TEXT,
    confidence_score INTEGER,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    locale TEXT,
    PRIMARY KEY (feed_id, locale)
);
```
Add the corresponding Rust struct in `models.rs`.
- [ ] **Step 4: Run test to verify it passes**
Run the database test. Expected: PASS.
- [ ] **Step 5: Commit**
`git add src/db/ tests/db_prices_test.rs && git commit -m "feat: add feed_prices table for caching scraper results"`

### Task 2: Create the Scraper Module

**Files:**
- Create: `src/scraper/mod.rs`
- Create: `src/scraper/marketplace.rs`
- Test: `tests/scraper_test.rs`

- [ ] **Step 1: Write the failing test**
Create a test that calls a `fetch_listings(feed_name, locale)` function and asserts it returns a list of results (Title, Price, URL).
- [ ] **Step 2: Run test to verify it fails**
Expected: FAIL (function not defined).
- [ ] **Step 3: Write minimal implementation**
Implement `fetch_listings` using `reqwest` and `scraper`. Make a search request to a source like Agroserver.ru or Flagma.by using the feed name. Parse the HTML to extract the top 3-5 listings.
- [ ] **Step 4: Run test to verify it passes**
Run the test (you may need to mock the HTTP response or use a live test if acceptable). Expected: PASS.
- [ ] **Step 5: Commit**
`git add src/scraper/ tests/scraper_test.rs && git commit -m "feat: implement HTML scraping for marketplace listings"`

### Task 3: Implement AI Filtering with Ollama

**Files:**
- Create: `src/ai/filter.rs`
- Test: `tests/ai_filter_test.rs`

- [ ] **Step 1: Write the failing test**
Write a test that passes mock listings to a `score_listings` function and expects a JSON array of scores.
- [ ] **Step 2: Run test to verify it fails**
Expected: FAIL.
- [ ] **Step 3: Write minimal implementation**
Implement `score_listings` using `reqwest` to POST to `http://localhost:11434/api/generate`. The prompt should instruct the model to rate the relevance of each listing to the target feed name from 1-10 and output only a JSON array of integers.
- [ ] **Step 4: Run test to verify it passes**
Run the test (mocking the Ollama response). Expected: PASS.
- [ ] **Step 5: Commit**
`git add src/ai/ tests/ai_filter_test.rs && git commit -m "feat: add Ollama integration for filtering scraped listings"`

### Task 4: Create the Axum API Endpoint

**Files:**
- Modify: `src/api/routes.rs`
- Create: `src/api/prices.rs`
- Test: `tests/api_prices_test.rs`

- [ ] **Step 1: Write the failing test**
Write an integration test for `GET /api/feeds/1004943047/price?locale=RU`.
- [ ] **Step 2: Run test to verify it fails**
Expected: FAIL (404 Not Found).
- [ ] **Step 3: Write minimal implementation**
Implement the endpoint in `src/api/prices.rs`.
1. Check the DB for a cached price < 7 days old. If found, return it.
2. If not, call `scraper::fetch_listings`.
3. Call `ai::score_listings`.
4. Filter out scores < 7.
5. Calculate the median price.
6. Insert into DB.
7. Return the price JSON.
- [ ] **Step 4: Run test to verify it passes**
Run the test. Expected: PASS.
- [ ] **Step 5: Commit**
`git add src/api/ tests/api_prices_test.rs && git commit -m "feat: implement JIT price scraping endpoint"`

### Task 5: Frontend Integration

**Files:**
- Modify: `frontend/src/api/feeds.ts`
- Modify: `frontend/src/components/FeedDetail.tsx` (or equivalent)
- Test: Manual UI verification (or update component tests if they exist).

- [ ] **Step 1: Add API client method**
Add `fetchFeedPrice(id: number, locale: string)` to `frontend/src/api/feeds.ts`.
- [ ] **Step 2: Update UI Component**
In the FeedDetail component, add a `useEffect` to call `fetchFeedPrice` when the component mounts.
Add a loading state (e.g., "Syncing Price...") to display while fetching.
Display the price, currency, and a link to the `source_url` once loaded. Fallback to "Price Unavailable" on error.
- [ ] **Step 3: Verify visually**
Run `npm run dev:full` and verify the UI updates correctly when viewing a feed.
- [ ] **Step 4: Commit**
`git add frontend/ && git commit -m "feat(ui): integrate JIT feed price syncing in feed details"`