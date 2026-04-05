# Felex Benchmark Supplementary

**Date:** 2026-03-29  
**Version:** 2.0

## Scope

Supplementary benchmark material for the regenerated 69 workflow runs.

## Canonical Artifacts

| Artifact | Path | Role |
|---|---|---|
| Primary results | `.claude/benchmarks/results/benchmark_results.json` | Full per-case/per-workflow metrics |
| Scenario-level summary | `.claude/benchmarks/data/csv/case_summary.csv` | Case aggregate view |
| Workflow quality table | `.claude/benchmarks/data/csv/workflow_quality.csv` | Workflow comparison |
| Species summary | `.claude/benchmarks/data/csv/species_summary.csv` | Species-level aggregation |
| Nutrient issue table | `.claude/benchmarks/data/csv/issue_summary.csv` | Deficiency/excess issue frequencies |
| Monogastric heatmap data | `.claude/benchmarks/data/csv/monogastric_heatmap_data.csv` | Heatmap input table |

## Regenerated Aggregate Snapshot

| Metric | Value |
|---|---:|
| Scenarios executed | 23 |
| Workflows per scenario | 3 |
| Total workflow runs | 69 |
| Mean runtime (`runtime_ms`) | 213.818 |
| Mean hard pass | 64.824 |
| Mean norm coverage index | 81.780 |
| Mean cost (RUB/day) | 183.007 |

## Workflow Snapshot

| Workflow | Runs | Mean runtime (`runtime_ms`) | Mean hard pass | Mean norm coverage |
|---|---:|---:|---:|---:|
| `build_from_library` | 23 | 534.449 | 66.037 | 83.208 |
| `complete_from_library` | 23 | 106.648 | 78.280 | 85.017 |
| `selected_only` | 23 | 0.358 | 50.156 | 77.116 |

## Reproduction

```bash
python .claude/benchmarks/scripts/export_data.py
python .claude/benchmarks/scripts/run_benchmark.py
python .claude/benchmarks/scripts/generate_comprehensive_figures.py
python .claude/benchmarks/scripts/generate_nutrient_summary.py
```

## Note

Historical supplementary prose and hand-typed per-scenario examples were removed in favor of artifact-backed summaries to keep benchmark interpretation reproducible and synchronized.
