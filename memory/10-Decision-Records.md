# 10 Decision Records

Updated: 2026-03-30
Owner: repository
Related: [[00-Index]], [[09-Change-Log]], [[11-Glossary]]
Tags: #memory #adr #decisions

## ADR-001 Local Structured Memory
Date: 2026-03-26
Status: Accepted

### Context
The repository needs durable project knowledge beyond transient chat context.

### Decision
Store project knowledge in linked Markdown notes under `memory/` and repeatable procedures under `skills/`.

### Consequences
- project knowledge becomes inspectable and versionable
- note quality becomes part of engineering hygiene
- tentative observations must be separated from canonical facts

### Related
- [[00-Index]]
- [[13-Operating-Rules]]

## ADR-002 Solver Workflow Semantics
Date: 2026-03-30
Status: Accepted

### Context
Repository prose historically mixed solver `mode` with library-completion workflow names.

### Decision
Treat the following as solve intents/workflows, not solver modes:
- `selected_only`
- `complete_from_library`
- `build_from_library`

Treat `mode` as the separate solver-strategy field exposed by `OptimizeRequest`.

### Consequences
- benchmark prose should describe these as workflows or intents
- manuscript text should not call them optimization modes unless it also explains the distinction

## ADR-003 Agent Backend and Context Defaults
Date: 2026-03-30
Status: Accepted

### Context
Legacy notes described the agent as Ollama-only with an `8192`-token default context.

### Decision
- default backend: `ollama`
- optional backend: `openai`-compatible path via `AgentConfig`
- default model: `qwen3.5:4b`
- supported heavier local model: `qwen3.5:9b`
- runtime default context size: `16384`
- context can be overridden through `FELEX_CONTEXT_SIZE`

### Consequences
- documentation should describe Ollama as the default local path, not the only backend
- publication prose must distinguish runtime defaults from benchmark-specific settings

## ADR-004 Benchmark Artifact Authority
Date: 2026-03-30
Status: Accepted

### Context
Legacy manuscript text drifted away from current benchmark artifacts.

### Decision
Benchmark metrics are authoritative only when tied to a specific artifact path and generation date.

### Consequences
- do not reuse historical headline numbers without checking the current artifact
- do not collapse raw feed-export counts and deduplicated benchmark-catalog counts into a single library-size claim
