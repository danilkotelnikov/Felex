# AGENTS.md - Coding Agent Guidelines

## Project Overview

**Felex** is a hybrid Rust + React + Tauri desktop application for animal feed ration calculation with a Python data pipeline.

| Layer | Technology | Location |
|-------|------------|----------|
| Backend | Rust (Axum, SQLite, good_lp) | `src/` |
| Frontend | React + TypeScript + Vite | `frontend/src/` |
| Desktop | Tauri wrapper | `src-tauri/` |
| Data Pipeline | Python (scraping, processing) | `database/` |
| Tests | Rust built-in, pytest | `tests/`, `database/tests/` |

---

## Build & Development Commands

### Prerequisites
- Node.js 18+
- Rust 1.70+
- Visual Studio Build Tools (C++ workload)
- Python 3.x (for data pipeline)

### Frontend (TypeScript/React)
```bash
npm run dev                    # Vite dev server (port 5173, proxies to :7432)
npm run dev:full               # Concurrent Axum + Vite
npm run build:feed-runtime     # Generate feed artifacts from database
npm run build                  # Full build: artifacts + tsc + Vite
npm run tauri:dev              # Tauri desktop dev mode
npm run tauri:build            # Build Tauri installers (NSIS/MSI)
npm run lint                   # ESLint TypeScript
npm run setup                  # PowerShell setup script
```

### Backend (Rust)
```bash
cargo run --bin felex-server   # Run backend API server
cargo run --bin migrate        # Run database migrations
cargo run --bin import-feeds   # Import feeds
cargo test                     # Run all Rust tests
cargo test <name>              # Run single test by name
```

### Python Data Pipeline
```bash
cd database
pip install -r requirements.txt
python -m pytest               # Run all Python tests
python -m pytest <file>        # Run single test file
python -m pytest -k <name>     # Run tests matching keyword
```

---

## Code Style Guidelines

### TypeScript/React
- **Indentation:** 2 spaces
- **Quotes:** Single quotes
- **Semicolons:** Required
- **File extensions:** `.ts` modules, `.tsx` components
- **Naming:**
  - Components: `PascalCase` (e.g., `NutrientPanel`)
  - Hooks: `useX.ts` pattern (e.g., `useRation.ts`)
  - Stores: `*Store.ts` (e.g., `rationStore.ts`)
  - Utils: `camelCase` or `kebab-case`
- **Imports:** Path alias `@/*` → `frontend/src/*`
- **Styling:** Tailwind CSS with `cn()` utility (clsx + tailwind-merge)
- **State:** Zustand stores
- **Error handling:** Try-catch, optional chaining, type guards
- **TypeScript:** Strict mode enabled (`noUnusedLocals`, `noUnusedParameters`)

### Rust
- **Indentation:** 4 spaces (rustfmt defaults)
- **Naming:**
  - Modules: `snake_case` (e.g., `diet_engine`)
  - Functions: `snake_case` (e.g., `create_router`)
  - Types/Structs: `PascalCase` (e.g., `AppState`)
  - Traits: `PascalCase`
- **Error handling:** `anyhow::Result<T>`, `?` operator, `thiserror` for custom errors
- **Logging:** `tracing` crate with `tracing-subscriber`
- **Async:** `tokio` runtime, `async/await`
- **Tests:** `#[cfg(test)]` modules, `#[tokio::test]` for async tests

### Python
- **Style:** PEP 8 with type hints
- **Async:** `asyncio` with `aiohttp`, `pytest-asyncio`
- **Validation:** `pydantic` models

---

## Testing Guidelines

### Running Single Tests
```bash
# Rust - by name pattern
cargo test optimizer           # Run tests containing "optimizer"
cargo test --lib diet_engine   # Run module tests

# Python - by file or keyword
cd database && python -m pytest tests/test_translator.py -v
python -m pytest -k "integration"  # Keyword match
```

### Adding Tests
- **Rust:** Add `#[test]` or `#[tokio::test]` near changed modules
- **Python:** Add `database/tests/test_*.py` cases
- **Frontend:** No dedicated test suite; include manual verification steps

---

## Important Conventions

### Generated Artifacts
- Do not hand-edit files in `frontend/src/generated/`
- Regenerate via `npm run build:feed-runtime`

### Data Authority
- Feed data source: `database/output/` (NOT `frontend/src/data/comprehensive-feeds.json`)
- Backend-resolved norms are authoritative
- Distinguish raw authority exports from deduplicated runtime or benchmark catalogs when reporting feed-library counts

### Localization
- Zero hardcoded UI strings
- Use `useTranslationWithFallback()` for auto-fallback
- Add BOTH `ru.json` and `en.json` keys for every new string

### Build Output
- Cargo defaults to `C:\FelexBuild\target`; if unavailable, set `CARGO_TARGET_DIR=.\tmp-cargo-target`
- Treat `dist/`, `target/`, `tmp-cargo-target/`, `release-artifacts/` as generated

### Runtime Truths
- In Tauri production, the frontend primarily talks to the embedded Axum API over `http://localhost:7432/api/v1`; do not describe the main UI/backend path as pure Tauri IPC
- Tauri commands are mainly used for bootstrap and desktop-only utilities such as export/open-url
- The agent backend defaults to local Ollama/Qwen, but the codebase also supports an OpenAI-compatible backend via config/env
- `build_from_library`, `complete_from_library`, and `selected_only` are solve intents/workflows, not solver modes

### Benchmark Authority
- Manuscript or documentation metrics must cite the specific benchmark artifact path/date they come from
- Do not reuse legacy publication numbers without checking the current artifact schema and values first

---

## Commit Guidelines

- Use short imperative subjects (Conventional Commit style)
- Example: `fix: preserve backend norm authority in optimize dialog`
- Include affected areas, commands run, regenerated artifacts
- Include screenshots for UI changes

---

## Related Documentation

- `CLAUDE.md` - Controller for Claude Code behavior
- `memory/00-Index.md` - Project knowledge vault index
- `memory/13-Developer-Operations.md` - Detailed build/test procedures
- `.codex/2026-03-14-project-context-handoff.md` - Current product state

---

## No Cursor/Copilot Rules

This repository has no `.cursorrules`, `.cursor/rules/`, or `.github/copilot-instructions.md` files.
