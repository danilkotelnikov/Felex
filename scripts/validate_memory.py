#!/usr/bin/env python3
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MEMORY = ROOT / "memory"
SKILLS = ROOT / "skills"
INDEX = MEMORY / "00-Index.md"

META_FIELDS = ("Updated:", "Owner:", "Related:", "Tags:")
LINK_RE = re.compile(r"\[\[([^\]]+?)\]\]")

def parse_links(text: str):
    links = []
    for raw in LINK_RE.findall(text):
        target = raw.split("#", 1)[0].split("|", 1)[0].strip()
        if target:
            links.append(target)
    return links

def check_file(path: Path):
    errors = []
    text = path.read_text(encoding="utf-8")
    head = "\n".join(text.splitlines()[:12])

    for field in META_FIELDS:
        if field not in head:
            errors.append(f"missing metadata field {field}")

    links = parse_links(text)
    if path.stem not in {"00-Index", "90-Graph-Report"} and len(set(links)) < 2:
        errors.append("fewer than two wiki links found")

    if path.parent.name == "memory" and path.stem not in {"14-Session-Inbox", "90-Graph-Report"}:
        if "## " not in text:
            errors.append("no section headings found")

    return errors

def main() -> None:
    problems = []
    index_text = INDEX.read_text(encoding="utf-8")
    index_links = set(parse_links(index_text))

    for folder in (MEMORY, SKILLS):
        for path in sorted(folder.glob("*.md")):
            errors = check_file(path)
            for error in errors:
                problems.append(f"{path}: {error}")

    for path in sorted(MEMORY.glob("*.md")):
        if path.stem in {"00-Index", "90-Graph-Report"}:
            continue
        if path.stem not in index_links:
            problems.append(f"{path}: not linked from memory/00-Index.md")

    if problems:
        print("validation failed")
        for problem in problems:
            print(f"- {problem}")
        sys.exit(1)

    print("validation passed")

if __name__ == "__main__":
    main()
