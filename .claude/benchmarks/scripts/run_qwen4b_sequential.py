#!/usr/bin/env python3
"""
Qwen 3.5 4B Sequential Agentic Evaluation
Resource-conscious: processes one scenario at a time, 4096 token context limit
"""

import json
import requests
import time
from datetime import datetime
from pathlib import Path

OLLAMA_URL = "http://localhost:11434/api/generate"
CONTEXT_LIMIT = 4096
MODEL = "qwen3.5:4b"

# Output directory
OUTPUT_DIR = Path(__file__).parent.parent / "data" / "json" / "ollama_responses"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Benchmark data
BENCHMARK_FILE = Path(__file__).parent.parent / "results" / "benchmark_results.json"

# Selected scenarios for agentic evaluation (6 representative cases)
SCENARIOS = [
    "cattle_dairy_fresh",
    "cattle_dairy_early_lact",
    "cattle_dairy_heifer_12_18",
    "poultry_broiler_grower",
    "poultry_broiler_starter",
    "swine_finisher",
]

def load_benchmark():
    with open(BENCHMARK_FILE, 'r', encoding='utf-8') as f:
        return json.load(f)

def build_prebuild_prompt(scenario_label, species, top_feeds):
    """Build pre-build screening prompt (within 4096 tokens)."""
    feed_list = "\n".join([f"- {f}" for f in top_feeds[:15]])
    
    return f"""You are a feed ration formulation expert.

ANIMAL: {scenario_label}
SPECIES: {species}

TASK: Recommend 5-7 starter feeds for a balanced ration.

AVAILABLE FEEDS (top 15 by relevance):
{feed_list}

Consider:
1. Energy and protein balance
2. Key minerals (Ca, P)
3. Palatability and safety
4. Cost efficiency

Format:
1. Feed name - Reason (key nutrients)
2. ...

Keep response concise."""

def build_postbuild_prompt(scenario_label, species, feeds, nutrients):
    """Build post-build validation prompt (within 4096 tokens)."""
    feed_str = "\n".join([f"- {f['feed_name']}: {f['amount_kg']:.2f} kg" for f in feeds[:8]])
    nutrient_str = "\n".join([
        f"- {k}: {v['actual']:.1f} (target: {v['target']:.1f})"
        for k, v in list(nutrients.items())[:8]
    ])
    
    return f"""You are a feed ration formulation expert.

ANIMAL: {scenario_label}
SPECIES: {species}

CURRENT RATION:
{feed_str}

NUTRIENT SUMMARY (actual vs target):
{nutrient_str}

TASK:
1. Identify deficiencies/excesses
2. Suggest 2-3 improvements
3. Comment on cost
4. Flag safety concerns

Format:
**Deficiencies:** ...
**Excesses:** ...
**Recommendations:** ...
**Safety:** ...

Keep response concise."""

def call_ollama(prompt, scenario_id, workflow):
    """Call Ollama API with retry logic."""
    payload = {
        "model": MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {
            "num_ctx": CONTEXT_LIMIT,
            "temperature": 0.7,
        }
    }
    
    for attempt in range(3):
        try:
            response = requests.post(OLLAMA_URL, json=payload, timeout=120)
            response.raise_for_status()
            result = response.json()
            
            if "error" in result:
                print(f"  Ollama error: {result['error']}")
                time.sleep(5)
                continue
            
            return result
            
        except requests.exceptions.Timeout:
            print(f"  Timeout (attempt {attempt+1}/3)")
            time.sleep(10)
        except requests.exceptions.RequestException as e:
            print(f"  Request error: {e}")
            time.sleep(5)
    
    return {"response": "", "eval_count": 0}

def save_response(scenario_id, workflow, prompt, response_text, metadata):
    """Save response in Obsidian markdown format."""
    filename = f"{workflow}_{scenario_id}.md"
    filepath = OUTPUT_DIR / filename
    
    content = f"""---
scenario: {scenario_id}
model: qwen3.5:4b
workflow: {workflow}
timestamp: {metadata['timestamp']}
context_tokens: {CONTEXT_LIMIT}
response_tokens: {metadata['response_tokens']}
duration_seconds: {metadata['duration']:.2f}
---

## Request

{prompt}

---

## Response

{response_text}

---

## Metadata

- Model: qwen3.5:4b
- Context limit: {CONTEXT_LIMIT} tokens
- Response tokens: {metadata['response_tokens']}
- Duration: {metadata['duration']:.2f}s
- Status: {"success" if response_text else "empty"}
"""
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"  Saved: {filename}")

def main():
    print("=" * 70)
    print("Qwen 3.5 4B Sequential Agentic Evaluation")
    print(f"Context limit: {CONTEXT_LIMIT} tokens")
    print(f"Scenarios: {len(SCENARIOS)}")
    print("=" * 70)
    
    # Load benchmark data
    data = load_benchmark()
    cases = {c['case']['id']: c for c in data['benchmark']['cases']}
    
    total_tasks = len(SCENARIOS) * 2  # prebuild + postbuild per scenario
    completed = 0
    
    for scenario_id in SCENARIOS:
        print(f"\n[{completed}/{total_tasks}] Scenario: {scenario_id}")
        
        if scenario_id not in cases:
            print(f"  Skipping - not in benchmark")
            continue
        
        case = cases[scenario_id]
        label = case['case']['label']
        species = case['case']['species']
        
        # Get workflows
        workflows = {wf['intent']: wf for wf in case.get('workflows', [])}
        build_wf = workflows.get('build_from_library', {})
        
        # Get feed list for pre-build
        top_feeds = [f.get('feed_name', 'Unknown') for f in build_wf.get('feeds', [])[:15]]
        if not top_feeds:
            # Use generic feed list from library
            top_feeds = ["Силос", "Сено", "Шрот", "Зерно", "Минералы"] * 3
        
        # === PRE-BUILD ===
        print(f"  [{completed}/{total_tasks}] Pre-build...")
        prebuild_prompt = build_prebuild_prompt(label, species, top_feeds)
        
        start = time.time()
        result = call_ollama(prebuild_prompt, scenario_id, "prebuild")
        duration = time.time() - start
        
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'response_tokens': result.get('eval_count', 0),
            'duration': duration,
        }
        
        save_response(scenario_id, "prebuild", prebuild_prompt, 
                     result.get('response', ""), metadata)
        completed += 1
        
        time.sleep(3)  # Cooldown between requests
        
        # === POST-BUILD ===
        print(f"  [{completed}/{total_tasks}] Post-build...")
        feeds = build_wf.get('feeds', [])
        
        # Build nutrient summary from top_issues
        nutrients = {}
        for issue in build_wf.get('top_issues', [])[:8]:
            key = issue.get('key', 'unknown')
            nutrients[key] = {
                'actual': issue.get('actual', 0),
                'target': issue.get('target', 1),
            }
        
        postbuild_prompt = build_postbuild_prompt(label, species, feeds, nutrients)
        
        start = time.time()
        result = call_ollama(postbuild_prompt, scenario_id, "postbuild")
        duration = time.time() - start
        
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'response_tokens': result.get('eval_count', 0),
            'duration': duration,
        }
        
        save_response(scenario_id, "postbuild", postbuild_prompt,
                     result.get('response', ""), metadata)
        completed += 1
        
        time.sleep(3)  # Cooldown
    
    print("\n" + "=" * 70)
    print(f"Complete! Generated {completed} responses")
    print(f"Output: {OUTPUT_DIR}")
    print("=" * 70)

if __name__ == '__main__':
    main()
