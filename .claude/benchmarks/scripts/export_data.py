#!/usr/bin/env python3
"""
Felex Benchmark Data Export Script
Exports feeds, norms, prices, and scenarios from Felex database to JSON and CSV.
"""

import sqlite3
import json
import csv
import os
from datetime import datetime
from pathlib import Path

# Configuration
DB_PATH = "../../../felex.db"
OUTPUT_DIR = "../data"
JSON_DIR = os.path.join(OUTPUT_DIR, "json")
CSV_DIR = os.path.join(OUTPUT_DIR, "csv")

# Ensure directories exist
os.makedirs(JSON_DIR, exist_ok=True)
os.makedirs(CSV_DIR, exist_ok=True)

def get_db_connection():
    """Connect to Felex database."""
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn

def export_feeds(conn):
    """Export all feeds with nutrients."""
    print("Exporting feeds...")
    
    cursor = conn.cursor()
    cursor.execute("""
        SELECT * FROM feeds WHERE verified = 1 OR verified IS NULL
        ORDER BY id
    """)
    
    feeds = []
    for row in cursor.fetchall():
        feed = dict(row)
        feeds.append(feed)
    
    # JSON export
    with open(os.path.join(JSON_DIR, "feeds_export.json"), 'w', encoding='utf-8') as f:
        json.dump(feeds, f, ensure_ascii=False, indent=2)
    
    # CSV export
    if feeds:
        keys = feeds[0].keys()
        with open(os.path.join(CSV_DIR, "feeds_export.csv"), 'w', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=keys)
            writer.writeheader()
            writer.writerows(feeds)
    
    print(f"  [OK] Exported {len(feeds)} feeds")
    return feeds

def export_priced_feeds(conn):
    """Export feeds with price data."""
    print("Exporting priced feeds...")
    
    cursor = conn.cursor()
    cursor.execute("""
        SELECT f.*, fp.price_rubles_per_ton, fp.price_date, fp.region, fp.source
        FROM feeds f
        LEFT JOIN feed_prices fp ON f.id = fp.feed_id
        WHERE fp.price_rubles_per_ton IS NOT NULL
        ORDER BY f.id
    """)
    
    priced = []
    for row in cursor.fetchall():
        feed = dict(row)
        priced.append(feed)
    
    # JSON export
    with open(os.path.join(JSON_DIR, "priced_feeds.json"), 'w', encoding='utf-8') as f:
        json.dump(priced, f, ensure_ascii=False, indent=2)
    
    # CSV export
    if priced:
        keys = priced[0].keys()
        with open(os.path.join(CSV_DIR, "priced_feeds.csv"), 'w', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=keys)
            writer.writeheader()
            writer.writerows(priced)
    
    print(f"  [OK] Exported {len(priced)} priced feeds")
    return priced

def export_animal_norms(conn):
    """Export animal norms and requirements."""
    print("Exporting animal norms...")
    
    cursor = conn.cursor()
    
    # Get animal groups
    cursor.execute("SELECT * FROM animal_groups ORDER BY id")
    groups = [dict(row) for row in cursor.fetchall()]
    
    # Get animal norms (if any exist)
    cursor.execute("SELECT * FROM animal_norms ORDER BY id")
    norms = [dict(row) for row in cursor.fetchall()]
    
    # Export groups
    with open(os.path.join(JSON_DIR, "animal_groups.json"), 'w', encoding='utf-8') as f:
        json.dump(groups, f, ensure_ascii=False, indent=2)
    
    with open(os.path.join(CSV_DIR, "animal_groups.csv"), 'w', newline='', encoding='utf-8') as f:
        if groups:
            writer = csv.DictWriter(f, fieldnames=groups[0].keys())
            writer.writeheader()
            writer.writerows(groups)
    
    # Export norms
    with open(os.path.join(JSON_DIR, "animal_norms_export.json"), 'w', encoding='utf-8') as f:
        json.dump(norms, f, ensure_ascii=False, indent=2)
    
    if norms:
        with open(os.path.join(CSV_DIR, "animal_norms_export.csv"), 'w', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=norms[0].keys())
            writer.writeheader()
            writer.writerows(norms)
    
    print(f"  [OK] Exported {len(groups)} groups, {len(norms)} norms")
    return groups, norms

def export_benchmark_scenarios():
    """Export benchmark scenario definitions."""
    print("Exporting benchmark scenarios...")
    
    scenarios = {
        "cattle_dairy": [
            {"id": "cattle_dairy_fresh", "label": "Fresh dairy cows", "stage": "0-30 DIM", "species": "cattle"},
            {"id": "cattle_dairy_early_lact", "label": "Early-lactation dairy cows", "stage": "30-100 DIM", "species": "cattle"},
            {"id": "cattle_dairy_mid_lact", "label": "Mid-lactation dairy cows", "stage": "100-200 DIM", "species": "cattle"},
            {"id": "cattle_dairy_late_lact", "label": "Late-lactation dairy cows", "stage": "200-305 DIM", "species": "cattle"},
            {"id": "cattle_dairy_dry_early", "label": "Dry cows, early dry period", "stage": "60 days pre-calving", "species": "cattle"},
            {"id": "cattle_dairy_dry_late", "label": "Dry cows, late dry period", "stage": "21 days pre-calving", "species": "cattle"},
            {"id": "cattle_dairy_heifer_6_12", "label": "Dairy heifers, 6-12 months", "stage": "Growing", "species": "cattle"},
            {"id": "cattle_dairy_heifer_12_18", "label": "Dairy heifers, 12-18 months", "stage": "Pre-breeding", "species": "cattle"},
            {"id": "cattle_dairy_heifer_18_24", "label": "Dairy heifers, 18-24 months", "stage": "Breeding/gestation", "species": "cattle"},
            {"id": "cattle_dairy_calf_pre", "label": "Pre-weaned calves", "stage": "0-2 months", "species": "cattle"},
            {"id": "cattle_dairy_calf_post", "label": "Post-weaned calves", "stage": "2-6 months", "species": "cattle"},
            {"id": "cattle_dairy_high_prod", "label": "High-production cows", "stage": "Peak", "species": "cattle"},
            {"id": "cattle_dairy_low_prod", "label": "Low-production cows", "stage": "Late", "species": "cattle"},
            {"id": "cattle_dairy_organic", "label": "Organic dairy", "stage": "Mixed", "species": "cattle"}
        ],
        "cattle_beef": [
            {"id": "cattle_beef_calf", "label": "Beef calves", "weight": "200-250 kg", "species": "cattle"},
            {"id": "cattle_beef_stocker", "label": "Beef stockers", "weight": "250-400 kg", "species": "cattle"},
            {"id": "cattle_beef_finisher", "label": "Beef finishers", "weight": "400-600 kg", "species": "cattle"},
            {"id": "cattle_beef_500", "label": "Beef cattle 500 kg", "weight": "500 kg", "species": "cattle"},
            {"id": "cattle_beef_700", "label": "Beef cattle 700 kg", "weight": "700 kg", "species": "cattle"},
            {"id": "cattle_beef_900", "label": "Beef cattle 900 kg", "weight": "900 kg", "species": "cattle"},
            {"id": "cattle_beef_1200", "label": "Beef cattle 1200 kg", "weight": "1200 kg", "species": "cattle"}
        ],
        "swine": [
            {"id": "swine_piglet_nursery", "label": "Nursery piglets", "weight": "7-25 kg", "species": "swine"},
            {"id": "swine_grower", "label": "Grower pigs", "weight": "25-60 kg", "species": "swine"},
            {"id": "swine_finisher", "label": "Finisher pigs", "weight": "60-120 kg", "species": "swine"},
            {"id": "swine_sow_gestating", "label": "Gestating sows", "weight": "Adult", "species": "swine"},
            {"id": "swine_sow_lactating", "label": "Lactating sows", "weight": "Adult", "species": "swine"}
        ],
        "poultry": [
            {"id": "poultry_broiler_starter", "label": "Broiler starter", "stage": "0-10 days", "species": "poultry"},
            {"id": "poultry_broiler_grower", "label": "Broiler grower", "stage": "11-24 days", "species": "poultry"},
            {"id": "poultry_broiler_finisher", "label": "Broiler finisher", "stage": "25+ days", "species": "poultry"},
            {"id": "poultry_layer_peak", "label": "Laying hens", "stage": "Peak", "species": "poultry"}
        ],
        "stress": [
            {"id": "stress_limited_5", "label": "Limited library (5 feeds)", "type": "constraint"},
            {"id": "stress_limited_10", "label": "Limited library (10 feeds)", "type": "constraint"},
            {"id": "stress_no_price", "label": "No price data", "type": "economic"},
            {"id": "stress_high_cost", "label": "High-cost feeds (2-3×)", "type": "economic"},
            {"id": "stress_tight_energy", "label": "Tight energy tolerance (±5%)", "type": "constraint"},
            {"id": "stress_tight_protein", "label": "Tight protein tolerance (±5%)", "type": "constraint"},
            {"id": "stress_conflicting", "label": "Conflicting constraints", "type": "infeasibility"},
            {"id": "stress_missing_ca", "label": "No calcium sources", "type": "nutrient_gap"},
            {"id": "stress_missing_aa", "label": "No amino acid data", "type": "nutrient_gap"},
            {"id": "stress_max_inclusion", "label": "All feeds at max inclusion", "type": "palatability"}
        ]
    }
    
    # Agentic evaluation subset (10-15 scenarios)
    agentic_subset = [
        "cattle_dairy_fresh",
        "cattle_dairy_early_lact",
        "cattle_dairy_heifer_12_18",
        "cattle_dairy_high_prod",
        "cattle_beef_finisher",
        "cattle_beef_700",
        "swine_piglet_nursery",
        "swine_finisher",
        "swine_sow_lactating",
        "poultry_broiler_grower",
        "poultry_layer_peak",
        "stress_limited_10",
        "stress_no_price",
        "stress_tight_energy",
        "stress_missing_ca"
    ]
    
    export_data = {
        "scenarios": scenarios,
        "agentic_subset": agentic_subset,
        "total_scenarios": sum(len(v) for v in scenarios.values()),
        "agentic_count": len(agentic_subset)
    }
    
    with open(os.path.join(JSON_DIR, "benchmark_scenarios.json"), 'w', encoding='utf-8') as f:
        json.dump(export_data, f, ensure_ascii=False, indent=2)
    
    print(f"  [OK] Exported {export_data['total_scenarios']} scenarios")
    print(f"  [OK] Agentic subset: {export_data['agentic_count']} scenarios")
    
    return export_data

def export_metadata():
    """Export benchmark metadata."""
    print("Exporting metadata...")
    
    metadata = {
        "export_timestamp": datetime.now().isoformat(),
        "database_path": DB_PATH,
        "output_directory": OUTPUT_DIR,
        "version": "1.0",
        "phase": "Phase 4 - Benchmark Data Export"
    }
    
    with open(os.path.join(JSON_DIR, "export_metadata.json"), 'w', encoding='utf-8') as f:
        json.dump(metadata, f, ensure_ascii=False, indent=2)
    
    print("  [OK] Exported metadata")
    return metadata

def main():
    """Main export function."""
    print("=" * 60)
    print("Felex Benchmark Data Export")
    print("=" * 60)
    print(f"Database: {DB_PATH}")
    print(f"Output: {OUTPUT_DIR}")
    print()
    
    # Connect to database
    conn = get_db_connection()
    
    try:
        # Run exports
        feeds = export_feeds(conn)
        priced = export_priced_feeds(conn)
        groups, norms = export_animal_norms(conn)
        scenarios = export_benchmark_scenarios()
        metadata = export_metadata()
        
        print()
        print("=" * 60)
        print("Export Summary")
        print("=" * 60)
        print(f"Feeds: {len(feeds)}")
        print(f"Priced feeds: {len(priced)}")
        print(f"Animal groups: {len(groups)}")
        print(f"Animal norms: {len(norms)}")
        print(f"Benchmark scenarios: {scenarios['total_scenarios']}")
        print(f"Agentic subset: {scenarios['agentic_count']}")
        print()
        print(f"JSON files: {JSON_DIR}/")
        print(f"CSV files: {CSV_DIR}/")
        print("=" * 60)
        
    finally:
        conn.close()

if __name__ == "__main__":
    main()
