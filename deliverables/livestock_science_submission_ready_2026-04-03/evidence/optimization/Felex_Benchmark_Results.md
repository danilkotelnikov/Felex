# Felex Benchmark Results

**Date:** 2026-03-29  
**Version:** 2.0  
**Source of truth:** `.claude/benchmarks/results/benchmark_results.json`

## Execution Summary

| Metric | Value |
|---|---|
| Executed scenarios | 23 |
| Workflow runs | 69 |
| Mean runtime (`runtime_ms`) | 213.818 |
| Mean hard pass rate | 64.824 |
| Mean norm coverage index | 81.780 |
| Mean cost (RUB/day) | 183.007 |

## Workflow-Level Aggregates

| Workflow | N | Mean runtime (`runtime_ms`) | Mean hard pass | Mean norm coverage | Mean cost (RUB/day) |
|---|---:|---:|---:|---:|---:|
| `build_from_library` | 23 | 534.449 | 66.037 | 83.208 | 165.024 |
| `complete_from_library` | 23 | 106.648 | 78.280 | 85.017 | 247.745 |
| `selected_only` | 23 | 0.358 | 50.156 | 77.116 | 136.251 |

## Figure Set (Regenerated)

English:
- `figures/en/figure_01_quality_points.png`
- `figures/en/figure_02_quality_delta.png`
- `figures/en/figure_03_runtime.png`
- `figures/en/figure_04_heatmap.png`
- `figures/en/figure_05_cost_scatter.png`

Russian:
- `figures/ru/figure_01_quality_points.png`
- `figures/ru/figure_02_quality_delta.png`
- `figures/ru/figure_03_runtime.png`
- `figures/ru/figure_04_heatmap.png`
- `figures/ru/figure_05_cost_scatter.png`

## Derived CSV Artifacts

- `data/csv/case_summary.csv`
- `data/csv/issue_summary.csv`
- `data/csv/species_summary.csv`
- `data/csv/workflow_quality.csv`
- `data/csv/monogastric_heatmap_data.csv`

## Notes on Interpretation

- Values above are directly computed from the 69 workflow records in the benchmark JSON.
- `runtime_ms` is reported exactly as emitted by the benchmark binary field name and is not renormalized in this document.
- Historical values from earlier prose revisions are superseded by this regenerated run.
