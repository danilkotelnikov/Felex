# 13 Developer Operations

Updated: 2026-03-29  
Owner: repository  
Related: [[00-Index]], [[01-System-Overview]], [[03-Data-Model]]  
Tags: #memory #operations #developer

## Daily Commands

```powershell
# Frontend/dev
npm run dev
npm run dev:full
npm run tauri:dev

# Build
npm run build:feed-runtime
npm run build

# Verification
npm run lint
cargo test
python scripts/validate_memory.py
python scripts/graph_report.py
python scripts/reindex.py
```

## Operational Verification Order

For cross-layer updates:
1. Verify source code behavior first (`src/*`).
2. Verify memory notes (`memory/*`).
3. Verify benchmark artifacts/scripts (`.claude/benchmarks/*`).
4. Verify technical/manuscript prose.

If prose disagrees with executable artifacts, executable artifacts are authoritative.

## Nutrient and Optimizer Change Checklist

When changing nutrient or optimization behavior:
1. Update schema/migrations only if storage truly changes.
2. Update `Feed` model mapping in `src/db/feeds.rs`.
3. Update optimizer objective keys and key canonicalization.
4. Update API request/response semantics when needed.
5. Update memory notes (`00`, `01`, `03`, `05`, `08`).
6. Regenerate benchmark outputs before changing benchmark prose.

## Builder and Alternatives Change Checklist

When changing ration construction behavior:
1. Update starter-plan logic (`auto_populate`, `feed_groups`, matrix constraints).
2. Keep intent semantics explicit (`selected_only`, `complete_from_library`, `build_from_library`).
3. Validate alternatives generation (`alternatives.rs`) and response payload shape.
4. Add or adjust tests in touched Rust modules.

## Memory Maintenance Procedure

1. Update canonical notes in place (do not create duplicate notes).
2. Keep `memory/14-Session-Inbox.md` tentative.
3. Run `validate_memory.py` and `graph_report.py`.
4. Run `reindex.py` to refresh `00-Index.md` auto-index block.

## Documentation and Audit Debt Policy

- Shorten legacy audit text after remediation.
- Mark completed items explicitly as done.
- Keep unresolved items concise with owner and scope.
- Mark deferrals as future-release constraints and do not execute them in current remediation pass.
