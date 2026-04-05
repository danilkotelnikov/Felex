# CLAUDE.md

Controller for repository behavior. Keep durable knowledge in `memory/`.
Keep repeatable procedures in `skills/`.

## Primary Role

For non-trivial tasks, combine:
1. user request
2. relevant source/config
3. minimum required `memory/` notes
4. minimum required `skills/` procedures

Use the smallest context that preserves correctness.

## Context Routing

### Trivial
Use local code/config only.

### Moderate
Read:
1. current request
2. touched source files
3. `memory/00-Index.md`
4. one or two directly relevant memory notes

### Cross-layer or risky
Use full read order:
1. request
2. relevant source/config
3. `memory/00-Index.md`
4. linked notes that affect correctness
5. relevant skills
6. only then execute

## Operational Truth Guardrails

### Nutrient and schema authority
- Treat DB-derived nutrient values as the only authority for metric statements.
- Do not assume a nutrient is operational because it exists in prose, UI labels, or legacy scripts.
- Before writing claims, verify end-to-end path: DB/storage -> model fields -> optimizer/objective keys -> API -> docs.
- If a DB field is not consumed by optimizer/build paths, document it explicitly as out-of-scope for current optimization behavior.
- Do not claim unsupported nutrient surfaces in prompts, docs, benchmarks, or manuscripts.

### Optimization, builder, and alternatives guardrails
- Distinguish clearly between:
  - optimizer objective keys and tier handling,
  - starter-plan/feed-matrix behavior for ration construction,
  - alternatives generation and selection boundaries.
- Do not merge these flows into one narrative.
- Any quality or capability claim must be tied to implemented API paths and executable scripts.

### Benchmark and publication guardrails
- Regenerated benchmark JSON/CSV/figures are canonical; prose must follow them.
- If script output and prose disagree, scripts/output win.
- Keep one active benchmark pipeline; label historical pipelines as legacy context.

## Memory and Evidence Rules

- Do not use `memory/14-Session-Inbox.md` as canonical truth.
- Promote inbox observations only when evidenced by code, schema/migrations, reproducible runtime/tests, maintained config/docs, or explicit user instruction.
- Preserve contradictions when relevant; do not flatten conflicts.
- Update `memory/09-Change-Log.md` when canonical reality changes.

## Technical Debt Compression Rules

When remediation is requested:
- Edit existing audit/docs/manuscript files in place.
- Mark completed items as done.
- Keep open issues concise and owned.
- Mark explicitly deferred items as future-release constraints that must not be executed now.
- Prefer deleting stale claims over preserving contradictory historical prose.

## Prohibited Actions

1. Do not invent schema, routes, behavior, or baselines.
2. Do not hide contradictions between code and documentation.
3. Do not treat TODOs, stale prose, or unresolved tickets as facts.
4. Do not expose secrets or private data in notes/responses.
5. Do not create orphan note links.

## Preferred Verification Commands

Run only what is required:

```bash
npm run lint
npm run build
cargo test
python scripts/validate_memory.py
python scripts/graph_report.py
python scripts/reindex.py
```

Updated: 2026-03-29
