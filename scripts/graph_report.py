#!/usr/bin/env python3
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MEMORY = ROOT / "memory"
SKILLS = ROOT / "skills"
OUTPUT = MEMORY / "90-Graph-Report.md"

LINK_RE = re.compile(r"\[\[([^\]]+?)\]\]")

def parse_links(text: str):
    results = []
    for raw in LINK_RE.findall(text):
        target = raw.split("#", 1)[0].split("|", 1)[0].strip()
        if target:
            results.append(target)
    return results

def safe_id(name: str) -> str:
    chars = []
    for ch in name:
        if ch.isalnum():
            chars.append(ch)
        else:
            chars.append("_")
    value = "".join(chars)
    if value and value[0].isdigit():
        value = "N_" + value
    return value

def load_docs(folder: Path):
    docs = {}
    for path in sorted(folder.glob("*.md")):
        docs[path.stem] = path.read_text(encoding="utf-8")
    return docs

def main() -> None:
    docs = {}
    docs.update(load_docs(MEMORY))
    docs.update(load_docs(SKILLS))

    edges = set()
    link_counts = {}
    for name, text in docs.items():
        links = parse_links(text)
        link_counts[name] = len(links)
        for target in links:
            if target in docs:
                edges.add((name, target))

    incoming = {name: 0 for name in docs}
    outgoing = {name: 0 for name in docs}
    for src, dst in edges:
        outgoing[src] += 1
        incoming[dst] += 1

    orphans = sorted(name for name in docs if incoming[name] == 0 and name not in {"00 Index", "90-Graph-Report"})
    isolated = sorted(name for name in docs if incoming[name] == 0 and outgoing[name] == 0)

    mermaid_lines = ["```mermaid", "graph TD"]
    for src, dst in sorted(edges):
        mermaid_lines.append(f"  {safe_id(src)}[{src}] --> {safe_id(dst)}[{dst}]")
    mermaid_lines.append("```")

    report = []
    report.append("# 90 Graph Report")
    report.append("")
    report.append("Updated: generated")
    report.append("Owner: scripts")
    report.append("Related: [[00-Index]], [[12-Dependency-Map]], [[13-Operating-Rules]]")
    report.append("Tags: #generated #graph #report")
    report.append("")
    report.append("## Summary")
    report.append(f"- documents scanned: {len(docs)}")
    report.append(f"- edges found: {len(edges)}")
    report.append("")
    report.append("## Orphans")
    if orphans:
        for name in orphans:
            report.append(f"- [[{name}]]")
    else:
        report.append("- none")
    report.append("")
    report.append("## Isolated")
    if isolated:
        for name in isolated:
            report.append(f"- [[{name}]]")
    else:
        report.append("- none")
    report.append("")
    report.append("## Link Counts")
    for name in sorted(docs):
        report.append(f"- [[{name}]]: outgoing={outgoing[name]}, incoming={incoming[name]}, raw_links={link_counts[name]}")
    report.append("")
    report.append("## Mermaid Graph")
    report.extend(mermaid_lines)
    report.append("")

    OUTPUT.write_text("\n".join(report), encoding="utf-8", newline="\n")
    print(f"wrote {OUTPUT}")

if __name__ == "__main__":
    main()
