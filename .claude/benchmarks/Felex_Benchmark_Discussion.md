# Felex Benchmark Discussion

**Date:** 2026-03-29  
**Version:** 2.0

## Interpretation Anchors

This discussion is constrained to regenerated benchmark outputs:
- `.claude/benchmarks/results/benchmark_results.json`
- `.claude/benchmarks/data/csv/*.csv`
- `.claude/benchmarks/figures/en/*`, `.claude/benchmarks/figures/ru/*`

## Main Observations

1. **Workflow behavior differs structurally.**  
   `build_from_library` has the highest mean runtime (534.449 ms) and mid-level hard pass (66.037), while `complete_from_library` is faster (106.648 ms) and has the highest hard pass (78.280).

2. **Selected-only runs are near-instant but less robust.**  
   `selected_only` mean runtime is 0.358 ms, with lower hard pass (50.156) and lower norm coverage (77.116). This is consistent with constrained search space from user-selected feeds.

3. **Aggregate benchmark quality is moderate, not near-perfect.**  
   Across 69 runs: mean hard pass 64.824 and mean norm coverage 81.780. This supports practical utility with clear room for optimization tuning.

4. **Cost-quality tradeoffs remain scenario-dependent.**  
   Mean cost differs by workflow (`build`: 165.024, `complete`: 247.745, `selected`: 136.251 RUB/day), so no single workflow dominates both cost and quality in every case.

## Boundaries and Limits

- Results cover 23 executed scenarios from the internal benchmark case list and should not be generalized beyond this set.
- Benchmark interpretation reflects currently implemented constraint tiers, feed-matrix behavior, and alternatives generation path.
- Supplementary local-model evaluation is informative but not part of the core optimization metric aggregate.

## Practical Implication

The benchmark supports Felex as an offline optimization tool with fast response times and meaningful constraint coverage under implemented workflows. The primary improvement direction is quality robustness for constrained-search scenarios rather than raw speed.
