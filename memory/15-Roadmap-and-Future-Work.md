# 15 Roadmap and Future Work

Updated: 2026-03-29  
Owner: repository  
Related: [[00-Index]], [[09-Change-Log]], [[10-Decision-Records]]  
Tags: #memory #roadmap #future

## In-Scope Next Improvements (after current consistency recovery)

1. Remove schema/model drift for nutrient-like columns that exist in migrations but are not wired into `Feed` + optimizer.
2. Expand test coverage for intent transitions and starter-plan edge cases.
3. Add script-level checks that fail if benchmark prose numbers diverge from regenerated CSV/JSON.
4. Add structured consistency checks for figure-path references in technical docs/manuscripts.

## Explicit Future-Release Constraints (do not execute now)

1. Do not add new nutrient features beyond currently implemented optimizer/API wiring.
2. Do not introduce parallel benchmark pipelines.
3. Do not make publication claims that are not backed by regenerated benchmark artifacts.
4. Do not expand agent capability claims beyond tool-verified data surfaces.

## Deferred Research Items

| Item | Why deferred |
|---|---|
| extended amino-acid modeling in optimizer | requires schema + norm + UI harmonization |
| advanced environmental objective terms | not part of current benchmarked implementation |
| richer economic uncertainty modeling | needs dedicated benchmark design and data inputs |
