#!/usr/bin/env python3
"""
Felex Benchmark Comprehensive Figure Generator v3
Repurposed from legacy benchmark approach with full nutrient data extraction.
Creates publication-quality figures matching legacy design patterns.
"""

import json
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

# Configuration
RESULTS_DIR = Path(__file__).parent.parent / "results"
FIGURES_DIR = Path(__file__).parent.parent / "figures"
DATA_DIR = Path(__file__).parent.parent / "data"

BENCHMARK_JSON = RESULTS_DIR / "benchmark_results.json"

# Create output directories
for lang in ['en', 'ru']:
    (FIGURES_DIR / lang).mkdir(parents=True, exist_ok=True)
(DATA_DIR / 'csv').mkdir(parents=True, exist_ok=True)

# Styling - match legacy approach
plt.rcParams.update({
    'font.size': 10,
    'font.family': 'DejaVu Sans',
    'pdf.fonttype': 42,
    'ps.fonttype': 42,
    'svg.fonttype': 'none',
    'axes.titlesize': 13,
    'axes.labelsize': 11,
    'legend.fontsize': 9,
    'xtick.labelsize': 9,
    'ytick.labelsize': 9,
    'axes.facecolor': '#f5f4f2',
    'figure.facecolor': 'white',
    'grid.color': '#dddddd',
    'grid.linestyle': '-',
    'grid.alpha': 0.8,
    'axes.spines.top': False,
    'axes.spines.right': False,
    'axes.edgecolor': '#666666',
})

# Legacy-inspired colors
build_color = '#a7cddc'
balanced_color = '#e4a7a7'
neg_color = '#e7b7b7'
pos_color = '#b7dfc6'
runtime_colors = {'build': '#b8d0e3', 'complete': '#e6ddb1', 'selected': '#c6ddb9'}
species_marker = {'cattle': 'o', 'swine': 's', 'poultry': '^'}
species_color = {'cattle': '#c9b7e4', 'swine': '#b9d9d7', 'poultry': '#e7d3a2'}

# Scenario labels
SHORT_EN = {
    'cattle_dairy_fresh': 'Dairy fresh',
    'cattle_dairy_early_lact': 'Dairy early lact.',
    'cattle_dairy_dry_early': 'Dry cows, early',
    'cattle_dairy_heifer_12_18': 'Heifers 12–18 m',
    'cattle_beef_stocker': 'Beef stocker',
    'cattle_beef_finisher': 'Beef finisher',
    'cattle_beef_500': 'Beef 500 kg',
    'cattle_beef_600': 'Beef 600 kg',
    'cattle_beef_700': 'Beef 700 kg',
    'cattle_beef_800': 'Beef 800 kg',
    'cattle_beef_900': 'Beef 900 kg',
    'cattle_beef_1000': 'Beef 1000 kg',
    'cattle_beef_1100': 'Beef 1100 kg',
    'cattle_beef_1200': 'Beef 1200 kg',
    'swine_piglet_nursery': 'Piglets nursery',
    'swine_grower': 'Swine grower',
    'swine_finisher': 'Swine finisher',
    'swine_sow_gestating': 'Sows gestating',
    'swine_sow_lactating': 'Sows lactating',
    'poultry_broiler_starter': 'Broiler starter',
    'poultry_broiler_grower': 'Broiler grower',
    'poultry_broiler_finisher': 'Broiler finisher',
    'poultry_layer_peak': 'Layers peak',
}

SHORT_RU = {
    'cattle_dairy_fresh': 'Свежеотельные',
    'cattle_dairy_early_lact': 'Ранняя лактация',
    'cattle_dairy_dry_early': 'Сухостой, ранний',
    'cattle_dairy_heifer_12_18': 'Тёлки 12-18 мес',
    'cattle_beef_stocker': 'Доращ. КРС',
    'cattle_beef_finisher': 'Откорм КРС',
    'cattle_beef_500': 'КРС 500 кг',
    'cattle_beef_600': 'КРС 600 кг',
    'cattle_beef_700': 'КРС 700 кг',
    'cattle_beef_800': 'КРС 800 кг',
    'cattle_beef_900': 'КРС 900 кг',
    'cattle_beef_1000': 'КРС 1000 кг',
    'cattle_beef_1100': 'КРС 1100 кг',
    'cattle_beef_1200': 'КРС 1200 кг',
    'swine_piglet_nursery': 'Поросята доращ.',
    'swine_grower': 'Свиньи рост',
    'swine_finisher': 'Свиньи откорм',
    'swine_sow_gestating': 'Свиноматки супорос.',
    'swine_sow_lactating': 'Свиноматки лактир.',
    'poultry_broiler_starter': 'Бройлеры старт',
    'poultry_broiler_grower': 'Бройлеры рост',
    'poultry_broiler_finisher': 'Бройлеры финиш',
    'poultry_layer_peak': 'Несушки пик',
}

NUTRIENT_NAMES_EN = {
    'energy_oe': 'Energy OE (MJ)',
    'crude_protein': 'Crude Protein (g)',
    'crude_protein_pct': 'Crude Protein (%)',
    'calcium': 'Calcium (g)',
    'calcium_pct': 'Calcium (%)',
    'phosphorus': 'Phosphorus (g)',
    'magnesium': 'Magnesium (g)',
    'lysine': 'Lysine (g)',
    'lysine_sid': 'Lysine (SID)',
    'lysine_sid_pct': 'Lysine (SID, %)',
    'methionine_cystine': 'Met+Cys (g)',
    'copper': 'Copper',
    'iron': 'Iron',
    'vit_e': 'Vitamin E',
    'zinc': 'Zinc',
}

NUTRIENT_NAMES_RU = {
    'energy_oe': 'Энергия ОЭ (МДж)',
    'crude_protein': 'Протеин (г)',
    'crude_protein_pct': 'Протеин (%)',
    'calcium': 'Кальций (г)',
    'calcium_pct': 'Кальций (%)',
    'phosphorus': 'Фосфор (г)',
    'magnesium': 'Магний (г)',
    'lysine': 'Лизин (г)',
    'lysine_sid': 'Лизин (СИД)',
    'lysine_sid_pct': 'Лизин (СИД, %)',
    'methionine_cystine': 'Мет+Цис (г)',
    'copper': 'Медь',
    'iron': 'Железо',
    'vit_e': 'Витамин E',
    'zinc': 'Цинк',
}


def load_benchmark():
    """Load and parse benchmark results."""
    print("Loading benchmark results...")
    obj = json.load(open(BENCHMARK_JSON, 'r', encoding='utf-8'))
    cases = obj['benchmark']['cases']
    
    case_rows = []
    issue_rows = []
    nutrient_rows = []
    
    for c in cases:
        case = c['case']
        lib = c['library']
        workflows = {wf['intent']: wf for wf in c.get('workflows', [])}
        
        build = workflows.get('build_from_library', {})
        complete = workflows.get('complete_from_library', {})
        selected = workflows.get('selected_only', {})
        
        # Case-level metrics
        case_rows.append({
            'case_id': case['id'],
            'label_en': case['label'],
            'species': case['species'],
            'allowed_feed_count': lib['allowed_feed_count'],
            'build_quality': build.get('norm_coverage_index', 0),
            'complete_quality': complete.get('norm_coverage_index', 0),
            'selected_quality': selected.get('norm_coverage_index', 0),
            'quality_delta': complete.get('norm_coverage_index', 0) - build.get('norm_coverage_index', 0),
            'build_adequacy': build.get('hard_pass_rate', 0),
            'complete_adequacy': complete.get('hard_pass_rate', 0),
            'build_runtime_s': build.get('runtime_ms', 0) / 1000.0,
            'complete_runtime_s': complete.get('runtime_ms', 0) / 1000.0,
            'selected_runtime_s': selected.get('runtime_ms', 0) / 1000.0,
            'build_cost': build.get('cost_per_day_rub', 0),
            'complete_cost': complete.get('cost_per_day_rub', 0),
            'selected_cost': selected.get('cost_per_day_rub', 0),
        })
        
        # Extract nutrient issues from all workflows
        for stage_name, stage in [('build', build), ('complete', complete), ('selected', selected)]:
            for issue in stage.get('top_issues', []):
                issue_rows.append({
                    'case_id': case['id'],
                    'species': case['species'],
                    'stage': stage_name,
                    'key': issue['key'],
                    'tier': issue.get('tier', 3),
                    'cumulative_severity': max(
                        issue.get('relative_deficit') or 0,
                        issue.get('relative_excess') or 0,
                        issue.get('relative_target_gap') or 0
                    ),
                    'actual': issue.get('actual', 0),
                    'target': issue.get('target', 0),
                })
    
    case_df = pd.DataFrame(case_rows)
    issue_df = pd.DataFrame(issue_rows)
    
    # Calculate total runtime
    case_df['total_runtime_s'] = (
        case_df['build_runtime_s'] + 
        case_df['complete_runtime_s'] + 
        case_df['selected_runtime_s']
    )
    
    print(f"  Loaded {len(case_df)} cases, {len(issue_df)} nutrient issues")
    return case_df, issue_df


def save_derived_data(case_df, issue_df):
    """Save derived CSV data."""
    print("Saving derived data...")
    
    # Case summary
    case_df.to_csv(DATA_DIR / 'csv' / 'case_summary.csv', index=False, encoding='utf-8')
    
    # Issue summary
    issue_df.to_csv(DATA_DIR / 'csv' / 'issue_summary.csv', index=False, encoding='utf-8')
    
    # Species summary
    species_df = case_df.groupby('species', as_index=False).agg(
        case_count=('case_id', 'count'),
        mean_build_quality=('build_quality', 'mean'),
        mean_complete_quality=('complete_quality', 'mean'),
        mean_allowed_feeds=('allowed_feed_count', 'mean'),
        min_complete_quality=('complete_quality', 'min'),
        max_complete_quality=('complete_quality', 'max'),
        mean_build_cost=('build_cost', 'mean'),
        mean_total_runtime=('total_runtime_s', 'mean'),
    )
    species_df.to_csv(DATA_DIR / 'csv' / 'species_summary.csv', index=False, encoding='utf-8')
    
    # Workflow summary
    workflow_df = case_df.melt(
        id_vars=['case_id', 'species'],
        value_vars=['build_quality', 'complete_quality', 'selected_quality'],
        var_name='workflow',
        value_name='quality'
    )
    workflow_df.to_csv(DATA_DIR / 'csv' / 'workflow_quality.csv', index=False, encoding='utf-8')
    
    # Nutrient heatmap data (monogastric focus)
    mono = issue_df[(issue_df['stage'] == 'complete') & (issue_df['species'].isin(['swine', 'poultry']))].copy()
    agg = mono.groupby('key', as_index=False)['cumulative_severity'].sum().sort_values('cumulative_severity', ascending=False)
    top_keys = agg['key'].head(12).tolist()
    
    heat = mono[mono['key'].isin(top_keys)].pivot_table(
        index='case_id', columns='key', values='cumulative_severity', aggfunc='sum', fill_value=0
    )
    
    row_order = [r for r in SHORT_EN.keys() if r in heat.index]
    col_order = [c for c in top_keys if c in heat.columns]
    heat = heat.reindex(index=row_order, columns=col_order)
    heat.to_csv(DATA_DIR / 'csv' / 'monogastric_heatmap_data.csv', encoding='utf-8')
    
    print(f"  Saved: case_summary.csv, issue_summary.csv, species_summary.csv, workflow_quality.csv, monogastric_heatmap_data.csv")


def make_figures(case_df, issue_df, lang='en'):
    """Create all publication-quality figures."""
    print(f"\nGenerating {lang.upper()} figures...")
    
    labels = SHORT_EN if lang == 'en' else SHORT_RU
    out_dir = FIGURES_DIR / lang
    
    # Sort by case ID for consistent ordering
    order = case_df['case_id'].tolist()
    df = case_df.copy()
    df['label'] = df['case_id'].map(lambda x: labels.get(x, x))
    df['order'] = pd.Categorical(df['case_id'], categories=order, ordered=True)
    df = df.sort_values('order')
    
    # === Figure 1: Quality Points (Build vs Complete) ===
    print("  Figure 1: Quality points...")
    fig, ax = plt.subplots(figsize=(8.2, 10))
    y = np.arange(len(df))
    
    ax.scatter(df['build_quality'], y, s=20, color=build_color, 
               edgecolor='#6f9bb0', linewidth=0.6, 
               label='Build' if lang == 'en' else 'Построение', alpha=0.8)
    ax.scatter(df['complete_quality'], y, s=20, marker='s', color=balanced_color, 
               edgecolor='#a36f6f', linewidth=0.6,
               label='Complete' if lang == 'en' else 'Дополнение', alpha=0.8)
    
    # Connect points
    for yi, b, c in zip(y, df['build_quality'], df['complete_quality']):
        ax.plot([b, c], [yi, yi], color='#d4d4d4', linewidth=1, zorder=0, alpha=0.7)
    
    ax.set_yticks(y)
    ax.set_yticklabels(df['label'], fontsize=8)
    ax.invert_yaxis()
    ax.grid(True, axis='x', alpha=0.6)
    ax.set_xlabel('Integral quality score' if lang == 'en' else 'Интегральный балл качества', fontsize=10)
    ax.set_ylabel('Benchmark case' if lang == 'en' else 'Контрольный сценарий', fontsize=10)
    ax.set_title('Starter-library quality and completion' if lang == 'en' else 'Качество построения и дополнения из библиотеки', 
                 fontsize=12, fontweight='bold', pad=15)
    ax.legend(frameon=False, loc='lower right')
    plt.tight_layout()
    plt.savefig(out_dir / 'figure_01_quality_points.png', dpi=300, bbox_inches='tight')
    plt.savefig(out_dir / 'figure_01_quality_points.pdf', bbox_inches='tight')
    plt.close(fig)
    
    # === Figure 2: Quality Delta (Effect of Completion) ===
    print("  Figure 2: Quality delta...")
    d2 = df.sort_values('quality_delta', ascending=True)
    fig, ax = plt.subplots(figsize=(8.2, 9.4))
    colors = [neg_color if v < 0 else pos_color for v in d2['quality_delta']]
    
    bars = ax.barh(np.arange(len(d2)), d2['quality_delta'], color=colors, 
                   edgecolor='#c9a9a9', linewidth=0.6, alpha=0.85)
    ax.axvline(0, color='#777777', linewidth=0.8)
    
    ax.set_yticks(np.arange(len(d2)))
    ax.set_yticklabels(d2['label'], fontsize=8)
    ax.grid(True, axis='x', alpha=0.6)
    ax.set_xlabel('Change in quality score' if lang == 'en' else 'Изменение балла качества', fontsize=10)
    ax.set_ylabel('Benchmark case' if lang == 'en' else 'Контрольный сценарий', fontsize=10)
    ax.set_title('Effect of library completion on quality' if lang == 'en' else 'Эффект дополнения из библиотеки на качество', 
                 fontsize=12, fontweight='bold', pad=15)
    plt.tight_layout()
    plt.savefig(out_dir / 'figure_02_quality_delta.png', dpi=300, bbox_inches='tight')
    plt.savefig(out_dir / 'figure_02_quality_delta.pdf', bbox_inches='tight')
    plt.close(fig)
    
    # === Figure 3: Runtime Decomposition (Stacked Bars) ===
    print("  Figure 3: Runtime decomposition...")
    d4 = df.sort_values('total_runtime_s', ascending=False)
    fig, ax = plt.subplots(figsize=(8.2, 9.6))
    y = np.arange(len(d4))
    left = np.zeros(len(d4))
    
    workflow_labels = {
        'build_runtime_s': 'Build' if lang == 'en' else 'Построение',
        'complete_runtime_s': 'Complete' if lang == 'en' else 'Дополнение',
        'selected_runtime_s': 'Selected' if lang == 'en' else 'Выбранное',
    }
    workflow_keys = {
        'build_runtime_s': 'build',
        'complete_runtime_s': 'complete',
        'selected_runtime_s': 'selected',
    }
    
    for col, key in [('build_runtime_s', 'build'), ('complete_runtime_s', 'complete'), ('selected_runtime_s', 'selected')]:
        ax.barh(y, d4[col], left=left, color=runtime_colors[workflow_keys[col]], 
                edgecolor='#888888', linewidth=0.4, label=workflow_labels[col], alpha=0.85)
        left += d4[col].to_numpy()
    
    ax.set_yticks(y)
    ax.set_yticklabels(d4['label'], fontsize=8)
    ax.invert_yaxis()
    ax.grid(True, axis='x', alpha=0.6)
    ax.set_xlabel('Runtime, s' if lang == 'en' else 'Время выполнения, с', fontsize=10)
    ax.set_ylabel('Benchmark case' if lang == 'en' else 'Контрольный сценарий', fontsize=10)
    ax.set_title('Runtime decomposition by benchmark case' if lang == 'en' else 'Разложение времени выполнения по сценариям', 
                 fontsize=12, fontweight='bold', pad=15)
    ax.legend(frameon=False, loc='lower right')
    plt.tight_layout()
    plt.savefig(out_dir / 'figure_03_runtime.png', dpi=300, bbox_inches='tight')
    plt.savefig(out_dir / 'figure_03_runtime.pdf', bbox_inches='tight')
    plt.close(fig)
    
    # === Figure 4: Nutrient Deficiency Heatmap (Monogastric) ===
    print("  Figure 4: Nutrient heatmap...")
    mono = issue_df[(issue_df['stage'] == 'complete') & (issue_df['species'].isin(['swine', 'poultry']))].copy()
    
    if len(mono) > 0:
        agg = mono.groupby('key', as_index=False)['cumulative_severity'].sum().sort_values('cumulative_severity', ascending=False)
        top_keys = agg['key'].head(12).tolist()
        
        heat = mono[mono['key'].isin(top_keys)].pivot_table(
            index='case_id', columns='key', values='cumulative_severity', aggfunc='sum', fill_value=0
        )
        
        row_order = [r for r in SHORT_EN.keys() if r in heat.index]
        col_order = [c for c in top_keys if c in heat.columns]
        heat = heat.reindex(index=row_order, columns=col_order)
        
        # Translate labels
        row_map = SHORT_RU if lang == 'ru' else SHORT_EN
        col_map = NUTRIENT_NAMES_RU if lang == 'ru' else NUTRIENT_NAMES_EN
        
        heat.index = [str(row_map.get(x, x) or x) for x in heat.index]
        heat.columns = [str(col_map.get(x, x) or x) for x in heat.columns]
        
        # Create custom colormap (pastel severity)
        cmap = LinearSegmentedColormap.from_list('pastel_severity', 
                                                  ['#f1efec', '#d5e3c2', '#e6d4a7', '#e7b7b7'])
        
        fig, ax = plt.subplots(figsize=(9.6, 6.2))
        im = ax.imshow(heat.values, aspect='auto', cmap=cmap, vmin=0, 
                       vmax=max(2.4, float(heat.values.max()) if len(heat.values) > 0 else 2.4))
        
        ax.set_yticks(np.arange(heat.shape[0]))
        ax.set_yticklabels(heat.index, fontsize=8)
        ax.set_xticks(np.arange(heat.shape[1]))
        ax.set_xticklabels(heat.columns, rotation=35, ha='right', fontsize=8)
        
        ax.set_title('Repeated limiting nutrients in swine and poultry' if lang == 'en' else 
                     'Повторяющиеся лимитирующие нутриенты у свиней и птицы', 
                     fontsize=12, fontweight='bold', pad=15)
        
        cbar = fig.colorbar(im, ax=ax, fraction=0.035, pad=0.02)
        cbar.set_label('Cumulative severity' if lang == 'en' else 'Накопленная выраженность', fontsize=9)
        
        for spine in ax.spines.values():
            spine.set_visible(False)
        
        plt.tight_layout()
        plt.savefig(out_dir / 'figure_04_heatmap.png', dpi=300, bbox_inches='tight')
        plt.savefig(out_dir / 'figure_04_heatmap.pdf', bbox_inches='tight')
        plt.close(fig)
    else:
        print("    Skipping heatmap - no monogastric data")
    
    # === Figure 5: Cost by Species (Scatter) ===
    print("  Figure 5: Cost by species...")
    fig, ax = plt.subplots(figsize=(9, 6))
    
    for sp in ['cattle', 'poultry', 'swine']:
        sub = df[df['species'] == sp]
        if len(sub) == 0:
            continue
        
        ax.scatter(sub['build_cost'], sub['complete_cost'], 
                   s=35, marker=species_marker[sp], 
                   color=species_color[sp], edgecolor='#777777', linewidth=0.6,
                   label={'cattle': 'Cattle' if lang == 'en' else 'КРС',
                          'poultry': 'Poultry' if lang == 'en' else 'Птица',
                          'swine': 'Swine' if lang == 'en' else 'Свиньи'}[sp],
                   alpha=0.7)
    
    # Add diagonal line (y=x)
    min_cost = min(df['build_cost'].min(), df['complete_cost'].min())
    max_cost = max(df['build_cost'].max(), df['complete_cost'].max())
    ax.plot([min_cost, max_cost], [min_cost, max_cost], '--', color='#888888', linewidth=1, alpha=0.6)
    
    ax.grid(True, alpha=0.5)
    ax.set_xlabel('Build cost (RUB/day)' if lang == 'en' else 'Стоимость построения (руб/день)', fontsize=10)
    ax.set_ylabel('Complete cost (RUB/day)' if lang == 'en' else 'Стоимость дополнения (руб/день)', fontsize=10)
    ax.set_title('Cost comparison: Build vs Complete workflows' if lang == 'en' else 
                 'Сравнение стоимости: Построение vs Дополнение', 
                 fontsize=12, fontweight='bold', pad=15)
    ax.legend(frameon=False, loc='upper left')
    plt.tight_layout()
    plt.savefig(out_dir / 'figure_05_cost_scatter.png', dpi=300, bbox_inches='tight')
    plt.savefig(out_dir / 'figure_05_cost_scatter.pdf', bbox_inches='tight')
    plt.close(fig)
    
    print(f"  Saved figures to: {out_dir}")


def main():
    """Main execution."""
    print("=" * 70)
    print("Felex Benchmark Comprehensive Figure Generator v3")
    print("Repurposed from legacy benchmark approach")
    print("=" * 70)
    
    # Load data
    case_df, issue_df = load_benchmark()
    
    # Save derived data
    save_derived_data(case_df, issue_df)
    
    # Generate figures for both languages
    make_figures(case_df, issue_df, lang='en')
    make_figures(case_df, issue_df, lang='ru')
    
    print("\n" + "=" * 70)
    print("Figure generation complete!")
    print(f"English: {FIGURES_DIR / 'en'}")
    print(f"Russian: {FIGURES_DIR / 'ru'}")
    print(f"Data: {DATA_DIR / 'csv'}")
    print("=" * 70)


if __name__ == '__main__':
    main()
