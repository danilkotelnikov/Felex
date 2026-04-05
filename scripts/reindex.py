#!/usr/bin/env python3
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MEMORY = ROOT / "memory"
INDEX = MEMORY / "00-Index.md"
START = "<!-- AUTO-INDEX:START -->"
END = "<!-- AUTO-INDEX:END -->"

def main() -> None:
    notes = []
    for path in sorted(MEMORY.glob("*.md")):
        if path.name in {"00-Index.md", "90-Graph-Report.md"}:
            continue
        notes.append(f"- [[{path.stem}]]")

    index_text = INDEX.read_text(encoding="utf-8")
    new_block = START + "\n" + "\n".join(notes) + "\n" + END
    pattern = re.compile(re.escape(START) + r".*?" + re.escape(END), re.S)
    if not pattern.search(index_text):
        # Keep the script resilient: create marker block on first run if it is missing.
        anchor = "## Note Registry"
        if anchor in index_text:
            updated = index_text.replace(anchor, f"{anchor}\n\n{new_block}", 1)
        else:
            updated = index_text.rstrip() + "\n\n## Note Registry\n\n" + new_block + "\n"
    else:
        updated = pattern.sub(new_block, index_text)
    INDEX.write_text(updated, encoding="utf-8", newline="\n")
    print(f"updated auto-index with {len(notes)} notes")

if __name__ == "__main__":
    main()
