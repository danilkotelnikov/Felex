# 05 API Surface

**Updated:** 2026-03-30  
**Owner:** repository  
**Related:** [[00-Index]], [[01-System-Overview]], [[08-User-Workflows]]  
**Tags:** #memory #api #integration

## Overview

- API router is defined in `src/lib.rs`.
- Main prefix is `/api/v1/*`.
- Health route is `/health` (outside `/api/v1`).
- Local desktop usage: no auth layer in current implementation.

## Route Groups

### Feeds

| Route | Method | Handler |
|---|---|---|
| `/feeds` | GET | `api::feeds::list_feeds` |
| `/feeds` | POST | `api::feeds::create_feed` |
| `/feeds/:id` | GET | `api::feeds::get_feed` |
| `/feeds/:id` | PUT | `api::feeds::update_feed` |
| `/feeds/:id` | DELETE | `api::feeds::delete_feed` |
| `/feeds/:id/price` | GET | `api::prices::get_jit_feed_price` |
| `/feeds/import/capru` | POST | `api::feeds::import_capru` |
| `/feeds/sync` | POST | `api::feeds::sync_feeds` |

### Rations and Optimization

| Route | Method | Handler | Notes |
|---|---|---|---|
| `/rations` | GET | `list_rations` | list |
| `/rations` | POST | `create_ration` | create |
| `/rations/:id` | GET | `get_ration` | fetch full ration |
| `/rations/:id` | PUT | `update_ration` | update |
| `/rations/:id/optimize` | POST | `optimize_ration` | intent-aware optimization |
| `/rations/:id/alternatives` | POST | `optimize_ration_alternatives` | explicit alternative set |
| `/rations/:id/auto-populate` | POST | `auto_populate_ration` | starter plan |
| `/rations/:id/screen` | POST | `screen_ration` | feed-set screening |
| `/rations/:id/nutrients` | GET | `get_nutrients` | nutrient summary |
| `/rations/:id/economics` | GET | `get_economics` | cost summary |

### Norms

| Route | Method | Handler |
|---|---|---|
| `/norms/:animal_group_id` | GET | `api::norms::get_norms` |
| `/norms/:animal_group_id/resolve` | POST | `api::norms::resolve_norms` |

### Animals

| Route | Method | Handler |
|---|---|---|
| `/animals` | GET | `api::animals::list_animals` |
| `/animals` | POST | `api::animals::create_animal` |
| `/animals/:id` | GET | `api::animals::get_animal` |

### Presets

| Route | Method | Handler |
|---|---|---|
| `/presets` | GET | `api::rations::list_presets` |

### Prices

| Route | Method | Handler |
|---|---|---|
| `/prices` | GET | `api::prices::list_prices` |
| `/prices/fetch` | POST | `api::prices::fetch_prices` |
| `/prices/:feed_id` | PUT | `api::prices::update_price` |
| `/prices/:feed_id/history` | GET | `api::prices::get_price_history` |

### Workspace

| Route | Method | Handler |
|---|---|---|
| `/workspace/tree` | GET | `api::workspace::get_tree` |
| `/workspace/folder` | POST | `api::workspace::create_folder` |
| `/workspace/ration` | GET | `api::workspace::get_ration` |
| `/workspace/ration` | POST | `api::workspace::create_ration` |
| `/workspace/ration` | PUT | `api::workspace::update_ration` |
| `/workspace/ration` | DELETE | `api::workspace::delete_ration` |
| `/workspace/rename` | POST | `api::workspace::rename_item` |
| `/workspace/config` | GET | `api::workspace::get_config` |
| `/workspace/config` | PUT | `api::workspace::update_config` |

### Agent

| Route | Method | Handler |
|---|---|---|
| `/agent/status` | GET | `api::agent::get_status` |
| `/agent/chat` | POST | `api::agent::chat` |
| `/agent/chat/stream` | POST | `api::agent::chat_stream` |
| `/agent/reload` | POST | `api::agent::reload` |

### App

| Route | Method | Handler |
|---|---|---|
| `/app/meta` | GET | `api::app::get_meta` |
| `/health` | GET | inline health handler |

## Optimization API Semantics

`OptimizeRequest` supports:
- `mode` (`tiered`, `single_pass`, `repair`, `balance`, `fixed`, `minimize_cost`)
- `intent` (`selected_only`, `complete_from_library`, `build_from_library`)
- norm overrides (`norms`, `norm_preset_id`, `animal_properties`)
- optional `available_feed_ids` library narrowing

Intent controls whether starter-plan logic is allowed and how working ration is prepared before solving.

Important distinction:
- manuscript and benchmark prose sometimes refer to `build`, `complete`, and `selected` as optimization "modes";
- in the implementation they are solve `intent` values;
- solver `mode` is a separate field controlling strategy inside the chosen intent path.

## Agent Route Notes

- `/agent/chat/stream` is currently a compatibility route name, not a true streaming transport.
- `api::agent::chat_stream` returns a final JSON payload and is documented in code as non-streaming behavior.
- Current agent config is not Ollama-only: backend selection comes from `AgentConfig` and supports an OpenAI-compatible path.

## Response Envelope

Most handlers return:

```json
{
  "data": {}
}
```

Errors use:

```json
{
  "error": "code",
  "message": "details"
}
```
