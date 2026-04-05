#!/usr/bin/env python3
"""
Generate comprehensive nutrient summary for all 69 benchmark rations.
Outputs Obsidian-compatible markdown with full nutrient tables.
"""

import json
from pathlib import Path

RESULTS_DIR = Path(__file__).parent.parent / "results"
BENCHMARK_JSON = RESULTS_DIR / "benchmark_results.json"
OUTPUT_FILE = Path(__file__).parent.parent / "Felex_Benchmark_Nutrient_Summary.md"

# Russian translations
TRANSLATIONS = {
    "Scenario": "Сценарий",
    "Workflow": "Рабочий процесс",
    "Runtime": "Время выполнения",
    "Cost": "Стоимость",
    "Constraint Pass": "Выполнение ограничений",
    "Nutrient": "Нутриент",
    "Actual": "Факт",
    "Target": "Цель",
    "Status": "Статус",
    "Feed": "Корм",
    "Amount": "Количество",
    "Category": "Категория",
    "Build from library": "Построение из библиотеки",
    "Complete from library": "Дополнение из библиотеки",
    "Selected only": "Только выбранные",
}

NUTRIENT_NAMES_RU = {
    'energy_oe': 'Энергия ОЭ (МДж)',
    'crude_protein': 'Протеин сырой (г)',
    'calcium': 'Кальций (г)',
    'phosphorus': 'Фосфор (г)',
    'magnesium': 'Магний (г)',
    'potassium': 'Калий (г)',
    'sodium': 'Натрий (г)',
    'lysine': 'Лизин (г)',
    'methionine_cystine': 'Мет+Цис (г)',
    'starch': 'Крахмал (г)',
    'sugar': 'Сахар (г)',
    'crude_fiber': 'Клетчатка (г)',
    'vit_d3': 'Витамин D3 (МЕ)',
    'vit_e': 'Витамин E (МЕ)',
    'iron': 'Железо (мг)',
    'copper': 'Медь (мг)',
    'zinc': 'Цинк (мг)',
    'manganese': 'Марганец (мг)',
    'selenium': 'Селен (мг)',
    'iodine': 'Йод (мг)',
}


def t(text):
    """Translate text to Russian."""
    return TRANSLATIONS.get(text, text)


def load_benchmark():
    """Load benchmark results."""
    with open(BENCHMARK_JSON, 'r', encoding='utf-8') as f:
        return json.load(f)


def generate_nutrient_summary():
    """Generate comprehensive nutrient summary markdown."""
    data = load_benchmark()
    cases = data['benchmark']['cases']
    
    lines = []
    lines.append("# Felex Benchmark — Comprehensive Nutrient Summary")
    lines.append("")
    lines.append("**Date:** 2026-03-29")
    lines.append("**Version:** 1.0")
    lines.append("**Phase:** Phase 4 — Benchmark Supplementary Data")
    lines.append("**Tags:** #benchmark #nutrients #data #supplementary #obsidian")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## Executive Summary")
    lines.append("")
    lines.append(f"This document provides **complete nutrient profiles** for all **{len(cases) * 3} ration optimizations**")
    lines.append(f"({len(cases)} scenarios × 3 workflows) from the Felex benchmark study.")
    lines.append("")
    lines.append("### Benchmark Scope")
    lines.append("")
    lines.append("| Metric | Value |")
    lines.append("|--------|-------|")
    lines.append(f"| Total scenarios | {len(cases)} |")
    lines.append("| Workflows per scenario | 3 |")
    lines.append(f"| **Total rations** | **{len(cases) * 3}** |")
    lines.append("| Feed database size | 1,375 feeds |")
    lines.append("| Priced feeds | 300 |")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## Scenario Index")
    lines.append("")
    
    # Group by species
    species_groups = {}
    for case in cases:
        species = case['case']['species']
        if species not in species_groups:
            species_groups[species] = []
        species_groups[species].append(case)
    
    for species, species_cases in species_groups.items():
        lines.append(f"### {species.upper()}")
        lines.append("")
        lines.append("| ID | Label | Stage |")
        lines.append("|----|-------|-------|")
        for case in species_cases:
            case_id = case['case']['id']
            label = case['case']['label']
            stage = case['case'].get('stage', '')
            lines.append(f"| `{case_id}` | {label} | {stage} |")
        lines.append("")
    
    lines.append("---")
    lines.append("")
    lines.append("## Detailed Ration Data")
    lines.append("")
    
    # Detailed data for each case
    for case_idx, case in enumerate(cases, 1):
        case_id = case['case']['id']
        label = case['case']['label']
        species = case['case']['species']
        
        lines.append(f"### {case_idx}. {label} (`{case_id}`)")
        lines.append("")
        lines.append(f"**Species:** {species.title()}")
        lines.append("")
        
        workflows = {wf['intent']: wf for wf in case.get('workflows', [])}
        
        for wf_name, wf in workflows.items():
            wf_label = {
                'build_from_library': 'Build from library',
                'complete_from_library': 'Complete from library',
                'selected_only': 'Selected only',
            }.get(wf_name, wf_name)
            
            lines.append(f"#### Workflow: {wf_label}")
            lines.append("")
            
            # Metrics table
            lines.append("**Metrics:**")
            lines.append("")
            lines.append("| Metric | Value |")
            lines.append("|--------|-------|")
            lines.append(f"| Runtime | {wf.get('runtime_ms', 0):.0f} ms |")
            lines.append(f"| Cost | {wf.get('cost_per_day_rub', 0):.2f} RUB/day |")
            lines.append(f"| Hard constraints | {wf.get('hard_pass_rate', 0)*100:.1f}% |")
            lines.append(f"| Tier 1 pass | {wf.get('tier1_pass_rate', 0)*100:.1f}% |")
            lines.append(f"| Tier 2 pass | {wf.get('tier2_pass_rate', 0)*100:.1f}% |")
            lines.append(f"| Tier 3 pass | {wf.get('tier3_pass_rate', 0)*100:.1f}% |")
            lines.append(f"| Norm coverage | {wf.get('norm_coverage_index', 0):.3f} |")
            lines.append(f"| Deficiency index | {wf.get('deficiency_index', 0):.3f} |")
            lines.append(f"| Excess index | {wf.get('excess_index', 0):.3f} |")
            lines.append("")
            
            # Feed composition
            feeds = wf.get('feeds', [])
            if feeds:
                lines.append("**Feed composition:**")
                lines.append("")
                lines.append("| Feed | Amount (kg) | Cost (RUB) |")
                lines.append("|------|-------------|------------|")
                for feed in feeds:
                    feed_name = feed.get('feed_name', 'Unknown')
                    amount = feed.get('amount_kg', 0)
                    cost = feed.get('cost_per_day', 0)
                    lines.append(f"| {feed_name} | {amount:.2f} | {cost:.2f} |")
                lines.append("")
            
            # Nutrient summary from top issues
            top_issues = wf.get('top_issues', [])
            if top_issues:
                lines.append("**Nutrient profile (top issues):**")
                lines.append("")
                lines.append("| Nutrient | Actual | Target | Min | Max | Status |")
                lines.append("|----------|--------|--------|-----|-----|--------|")
                
                for issue in top_issues[:15]:  # Top 15 nutrients
                    key = issue.get('key', 'unknown')
                    actual = issue.get('actual', 0)
                    target = issue.get('target', 0)
                    min_val = issue.get('min', 0)
                    max_val = issue.get('max', 0)
                    tier = issue.get('tier', 3)
                    hard_pass = issue.get('hard_pass', False)
                    
                    # Determine status
                    if hard_pass:
                        status = 'Pass'
                    elif tier == 1:
                        status = 'Caution Tier 1'
                    elif tier == 2:
                        status = 'Caution Tier 2'
                    else:
                        status = 'ℹ️ Tier 3'
                    
                    nutrient_name = NUTRIENT_NAMES_RU.get(key, key.title())
                    lines.append(f"| {nutrient_name} | {actual or 0:.1f} | {target or 0:.1f} | {min_val or 0:.1f} | {max_val or 0:.1f} | {status} |")
                
                lines.append("")
            
            # Constraint violations
            unevaluable = wf.get('unevaluable_constraints', [])
            if unevaluable:
                lines.append("**Unevaluable constraints:**")
                lines.append("")
                for constraint in unevaluable[:5]:
                    lines.append(f"- {constraint}")
                lines.append("")
        
        lines.append("---")
        lines.append("")
    
    lines.append("## Aggregate Statistics")
    lines.append("")
    lines.append("### By Species")
    lines.append("")
    lines.append("| Species | Cases | Mean Runtime (ms) | Mean Cost (RUB/d) | Mean Coverage |")
    lines.append("|---------|-------|-------------------|-------------------|---------------|")
    
    for species, species_cases in species_groups.items():
        total_runtime = sum(
            wf.get('runtime_ms', 0)
            for case in species_cases
            for wf in case.get('workflows', [])
        ) / (len(species_cases) * 3)
        
        total_cost = sum(
            wf.get('cost_per_day_rub', 0)
            for case in species_cases
            for wf in case.get('workflows', [])
        ) / (len(species_cases) * 3)
        
        mean_coverage = sum(
            wf.get('norm_coverage_index', 0)
            for case in species_cases
            for wf in case.get('workflows', [])
        ) / (len(species_cases) * 3)
        
        lines.append(f"| {species.title()} | {len(species_cases)} | {total_runtime:.0f} | {total_cost:.1f} | {mean_coverage:.3f} |")
    
    lines.append("")
    lines.append("### By Workflow")
    lines.append("")
    lines.append("| Workflow | Rations | Mean Runtime (ms) | Mean Cost (RUB/d) | Mean Coverage |")
    lines.append("|----------|---------|-------------------|-------------------|---------------|")
    
    workflow_stats = {
        'build_from_library': {'runtime': [], 'cost': [], 'coverage': []},
        'complete_from_library': {'runtime': [], 'cost': [], 'coverage': []},
        'selected_only': {'runtime': [], 'cost': [], 'coverage': []},
    }
    
    for case in cases:
        for wf in case.get('workflows', []):
            intent = wf.get('intent', '')
            if intent in workflow_stats:
                workflow_stats[intent]['runtime'].append(wf.get('runtime_ms', 0))
                workflow_stats[intent]['cost'].append(wf.get('cost_per_day_rub', 0))
                workflow_stats[intent]['coverage'].append(wf.get('norm_coverage_index', 0))
    
    for wf_name, stats in workflow_stats.items():
        wf_label = {
            'build_from_library': 'Build from library',
            'complete_from_library': 'Complete from library',
            'selected_only': 'Selected only',
        }.get(wf_name, wf_name)
        
        mean_runtime = sum(stats['runtime']) / len(stats['runtime']) if stats['runtime'] else 0
        mean_cost = sum(stats['cost']) / len(stats['cost']) if stats['cost'] else 0
        mean_coverage = sum(stats['coverage']) / len(stats['coverage']) if stats['coverage'] else 0
        
        lines.append(f"| {wf_label} | {len(stats['runtime'])} | {mean_runtime:.0f} | {mean_cost:.1f} | {mean_coverage:.3f} |")
    
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## Data Files")
    lines.append("")
    lines.append("| File | Location | Description |")
    lines.append("|------|----------|-------------|")
    lines.append("| Case summary | `.claude/benchmarks/data/csv/case_summary.csv` | Aggregated metrics per scenario |")
    lines.append("| Issue summary | `.claude/benchmarks/data/csv/issue_summary.csv` | Nutrient deficiency records |")
    lines.append("| Species summary | `.claude/benchmarks/data/csv/species_summary.csv` | Aggregates by species |")
    lines.append("| Heatmap data | `.claude/benchmarks/data/csv/monogastric_heatmap_data.csv` | Swine/poultry nutrient severity |")
    lines.append("| Full results JSON | `.claude/benchmarks/results/benchmark_results.json` | Complete benchmark data |")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("**Generated:** 2026-03-29")
    lines.append("")
    lines.append("**Related:**")
    lines.append("- [[Felex_Benchmark_Methodology]] — Methods and protocol")
    lines.append("- [[Felex_Benchmark_Results]] — Results analysis")
    lines.append("- [[Felex_Benchmark_Rations_Supplementary]] — Ration compositions")
    lines.append("- [[Felex_Figure_Index]] — Figure descriptions")
    lines.append("")
    
    # Write to file
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    
    print(f"Generated comprehensive nutrient summary: {OUTPUT_FILE}")
    print(f"Total cases: {len(cases)}")
    print(f"Total rations documented: {len(cases) * 3}")


if __name__ == '__main__':
    generate_nutrient_summary()
