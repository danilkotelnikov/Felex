#!/usr/bin/env python3
"""
Felex benchmark runner.

This wrapper executes the canonical Rust benchmark binary and validates the
result artifact shape instead of re-parsing console logs.
"""

from __future__ import annotations

import json
import statistics
import subprocess
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent.parent.parent
RESULTS_DIR = SCRIPT_DIR.parent / "results"
RESULTS_FILE = RESULTS_DIR / "benchmark_results.json"


def run_cmd(args: list[str], timeout_sec: int) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=timeout_sec,
    )


def benchmark_summary(payload: dict) -> str:
    cases = payload.get("benchmark", {}).get("cases", [])
    workflows = [wf for case in cases for wf in case.get("workflows", [])]
    if not workflows:
        return "No workflows found in benchmark output."

    runtime_ms = statistics.fmean(wf.get("runtime_ms", 0.0) for wf in workflows)
    hard_pass = statistics.fmean(wf.get("hard_pass_rate", 0.0) for wf in workflows)
    coverage = statistics.fmean(wf.get("norm_coverage_index", 0.0) for wf in workflows)
    cost = statistics.fmean(wf.get("cost_per_day_rub", 0.0) for wf in workflows)
    return (
        f"cases={len(cases)}, workflows={len(workflows)}, "
        f"mean_runtime_ms={runtime_ms:.3f}, "
        f"mean_hard_pass={hard_pass:.3f}, "
        f"mean_norm_coverage={coverage:.3f}, "
        f"mean_cost_rub={cost:.3f}"
    )


def main() -> int:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    print("Building benchmark binary...")
    build = run_cmd(["cargo", "build", "--release", "--bin", "run_publication_benchmark"], 900)
    if build.returncode != 0:
        print(build.stderr[:2000])
        print("Falling back to debug build...")
        build = run_cmd(["cargo", "build", "--bin", "run_publication_benchmark"], 900)
        if build.returncode != 0:
            print(build.stderr[:2000])
            return 1

    print("Running benchmark...")
    run = run_cmd(
        [
            "cargo",
            "run",
            "--release",
            "--bin",
            "run_publication_benchmark",
            "--",
            "--output-dir",
            str(RESULTS_DIR),
        ],
        1800,
    )
    if run.returncode != 0:
        print(run.stdout[:2000])
        print(run.stderr[:2000])
        return 1

    if not RESULTS_FILE.exists():
        print(f"Expected artifact not found: {RESULTS_FILE}")
        return 1

    payload = json.loads(RESULTS_FILE.read_text(encoding="utf-8"))
    print("Benchmark completed.")
    print(benchmark_summary(payload))
    return 0


if __name__ == "__main__":
    sys.exit(main())
