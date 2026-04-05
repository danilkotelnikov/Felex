#!/usr/bin/env python3
"""
Qwen 3.5 4B Final Evaluation with Continue Logic
Handles Qwen's thinking mode by requesting continuation
"""

import json
import requests
import time
from datetime import datetime
from pathlib import Path

OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "qwen3.5:4b"
CONTEXT_LIMIT = 4096

OUTPUT_DIR = Path(__file__).parent.parent / "data" / "json" / "ollama_responses"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

BENCHMARK_FILE = Path(__file__).parent.parent / "results" / "benchmark_results.json"

SCENARIOS = [
    "cattle_dairy_fresh",
    "cattle_beef_stocker", 
    "poultry_broiler_starter",
]

def load_benchmark():
    with open(BENCHMARK_FILE, 'r', encoding='utf-8') as f:
        return json.load(f)

def call_ollama_with_continue(prompt, max_attempts=3):
    """Call Ollama and request continuation if only thinking tokens produced."""
    
    for attempt in range(max_attempts):
        payload = {
            "model": MODEL,
            "prompt": prompt,
            "stream": False,
            "options": {"num_ctx": CONTEXT_LIMIT, "temperature": 0.7}
        }
        
        try:
            response = requests.post(OLLAMA_URL, json=payload, timeout=300)
            result = response.json()
            
            response_text = result.get('response', '').strip()
            tokens = result.get('eval_count', 0)
            
            # If response is empty but tokens were generated, request continuation
            if not response_text and tokens > 100:
                print(f"    Thinking mode detected ({tokens} tokens), requesting continuation...")
                prompt = "Continue from the reasoning you already completed. Provide ONLY the final answer in the requested format. Do not repeat the reasoning."
                continue
            
            return response_text, tokens
            
        except Exception as e:
            print(f"    Error: {e}")
            time.sleep(5)
    
    return "", 0

def main():
    print("=" * 70)
    print("Qwen 3.5 4B Final Evaluation (with continue logic)")
    print(f"Scenarios: {len(SCENARIOS)}")
    print("=" * 70)
    
    data = load_benchmark()
    cases = {c['case']['id']: c for c in data['benchmark']['cases']}
    
    total = len(SCENARIOS) * 2
    completed = 0
    
    for scenario_id in SCENARIOS:
        print(f"\n[{completed}/{total}] {scenario_id}")
        
        if scenario_id not in cases:
            continue
        
        case = cases[scenario_id]
        label = case['case']['label']
        species = case['case']['species']
        
        workflows = {wf['intent']: wf for wf in case.get('workflows', [])}
        build_wf = workflows.get('build_from_library', {})
        
        # PRE-BUILD
        print(f"  [{completed}/{total}] Pre-build...")
        feeds = build_wf.get('feeds', [])[:5]
        feed_names = ", ".join([f.get('feed_name', '?')[:15] for f in feeds])
        
        prompt = f"""Ration for {label} ({species}).

Available: {feed_names or 'Standard feeds'}.

Recommend 5 feeds for balanced ration.
Consider: energy, protein, minerals, cost.

Format: 1) Feed - Reason"""
        
        start = time.time()
        response_text, tokens = call_ollama_with_continue(prompt)
        duration = time.time() - start
        
        print(f"    Duration: {duration:.1f}s, Tokens: {tokens}, Response: {len(response_text)} chars")
        
        # Save
        metadata = {'timestamp': datetime.now().isoformat(), 'response_tokens': tokens, 'duration': duration}
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

{prompt}

---

## Response

{response_text}

---

## Metadata

- Duration: {duration:.2f}s
- Tokens: {tokens}
- Status: {"success" if response_text else "empty"}
""")
        
        completed += 1
        time.sleep(3)
        
        # POST-BUILD
        print(f"  [{completed}/{total}] Post-build...")
        feeds = build_wf.get('feeds', [])[:8]
        feed_str = "\n".join([f"- {f.get('feed_name', '?')}: {f.get('amount_kg', 0):.2f} kg" for f in feeds])
        
        issues = build_wf.get('top_issues', [])[:8]
        nutrient_str = "\n".join([f"- {i.get('key', '?')}: {i.get('actual', 0):.1f} (target: {i.get('target', 1):.1f})" for i in issues])
        
        prompt = f"""Evaluate ration for {label}:

Feeds:
{feed_str}

Nutrients:
{nutrient_str}

Task:
1. Deficiencies?
2. Improvements (2)?
3. Cost comment?
4. Safety concerns?

Format: Brief bullet points."""
        
        start = time.time()
        response_text, tokens = call_ollama_with_continue(prompt)
        duration = time.time() - start
        
        print(f"    Duration: {duration:.1f}s, Tokens: {tokens}, Response: {len(response_text)} chars")
        
        # Save
        metadata = {'timestamp': datetime.now().isoformat(), 'response_tokens': tokens, 'duration': duration}
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

{prompt}

---

## Response

{response_text}

---

## Metadata

- Duration: {duration:.2f}s
- Tokens: {tokens}
- Status: {"success" if response_text else "empty"}
""")
        
        completed += 1
        time.sleep(3)
    
    print("\n" + "=" * 70)
    print(f"Complete! {completed} responses generated")
    print("=" * 70)

if __name__ == '__main__':
    main()
