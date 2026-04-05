#!/usr/bin/env python3
"""
Qwen 3.5 4B Simple Sequential Evaluation
Short prompts, 6 scenarios, resource-conscious
"""

import json
import requests
import time
from datetime import datetime
from pathlib import Path

OLLAMA_URL = "http://localhost:11434/api/generate"
CONTEXT_LIMIT = 4096
MODEL = "qwen3.5:4b"

OUTPUT_DIR = Path(__file__).parent.parent / "data" / "json" / "ollama_responses"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

BENCHMARK_FILE = Path(__file__).parent.parent / "results" / "benchmark_results.json"

# 6 representative scenarios
SCENARIOS = [
    "cattle_dairy_fresh",
    "cattle_beef_stocker",
    "poultry_broiler_starter",
]

def load_benchmark():
    with open(BENCHMARK_FILE, 'r', encoding='utf-8') as f:
        return json.load(f)

def call_ollama(prompt, timeout=180):
    """Call Ollama with extended timeout."""
    payload = {
        "model": MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {"num_ctx": CONTEXT_LIMIT, "temperature": 0.7}
    }
    
    try:
        response = requests.post(OLLAMA_URL, json=payload, timeout=timeout)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        return {"error": str(e), "response": ""}

def main():
    print("=" * 60)
    print("Qwen 3.5 4B Simple Evaluation (6 tasks)")
    print("=" * 60)
    
    data = load_benchmark()
    cases = {c['case']['id']: c for c in data['benchmark']['cases']}
    
    for scenario_id in SCENARIOS:
        print(f"\n=== {scenario_id} ===")
        
        if scenario_id not in cases:
            continue
        
        case = cases[scenario_id]
        label = case['case']['label']
        species = case['case']['species']
        
        workflows = {wf['intent']: wf for wf in case.get('workflows', [])}
        build_wf = workflows.get('build_from_library', {})
        
        # PRE-BUILD
        print("Pre-build...")
        feeds = build_wf.get('feeds', [])[:5]
        feed_names = ", ".join([f.get('feed_name', '?')[:15] for f in feeds])
        
        pre_prompt = f"""Ration for {label} ({species}).

Available: {feed_names or 'Standard feeds'}.

Recommend 5 feeds for balanced ration.
Consider: energy, protein, minerals, cost.

Response format: 1) Feed - Reason"""
        
        start = time.time()
        result = call_ollama(pre_prompt, timeout=180)
        duration = time.time() - start
        
        response_text = result.get('response', '')
        tokens = result.get('eval_count', 0)
        
        print(f"  Duration: {duration:.1f}s, Tokens: {tokens}")
        
        # Save pre-build
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'response_tokens': tokens,
            'duration': duration,
        }
        
        filepath = OUTPUT_DIR / f"qwen4b_prebuild_{scenario_id}.md"
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(f"""---
scenario: {scenario_id}
model: qwen3.5:4b
workflow: prebuild
timestamp: {metadata['timestamp']}
context_tokens: {CONTEXT_LIMIT}
response_tokens: {tokens}
duration_seconds: {duration:.2f}
---

## Request

{pre_prompt}

---

## Response

{response_text}

---

## Metadata

- Duration: {duration:.2f}s
- Tokens: {tokens}
- Status: {"success" if response_text else "empty"}
""")
        
        time.sleep(5)
        
        # POST-BUILD
        print("Post-build...")
        feeds = build_wf.get('feeds', [])[:5]
        feed_str = "\n".join([f"- {f.get('feed_name', '?')}: {f.get('amount_kg', 0):.2f} kg" for f in feeds])
        
        issues = build_wf.get('top_issues', [])[:5]
        nutrient_str = "\n".join([f"- {i.get('key', '?')}: {i.get('actual', 0):.1f} (target: {i.get('target', 1):.1f})" for i in issues])
        
        post_prompt = f"""Evaluate ration for {label}:

Feeds:
{feed_str}

Nutrients:
{nutrient_str}

Task:
1. Deficiencies?
2. Improvements (2)?
3. Cost comment?

Format: Brief bullet points."""
        
        start = time.time()
        result = call_ollama(post_prompt, timeout=180)
        duration = time.time() - start
        
        response_text = result.get('response', '')
        tokens = result.get('eval_count', 0)
        
        print(f"  Duration: {duration:.1f}s, Tokens: {tokens}")
        
        # Save post-build
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'response_tokens': tokens,
            'duration': duration,
        }
        
        filepath = OUTPUT_DIR / f"qwen4b_postbuild_{scenario_id}.md"
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(f"""---
scenario: {scenario_id}
model: qwen3.5:4b
workflow: postbuild
timestamp: {metadata['timestamp']}
context_tokens: {CONTEXT_LIMIT}
response_tokens: {tokens}
duration_seconds: {duration:.2f}
---

## Request

{post_prompt}

---

## Response

{response_text}

---

## Metadata

- Duration: {duration:.2f}s
- Tokens: {tokens}
- Status: {"success" if response_text else "empty"}
""")
        
        time.sleep(5)
    
    print("\n" + "=" * 60)
    print("Complete!")
    print("=" * 60)

if __name__ == '__main__':
    main()
