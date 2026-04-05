"""Output generator for the feed database."""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import jsonschema

from config import FINAL_DB_FILE, OUTPUT_DIR, SCHEMA_FILE
from models import FeedDatabase, FeedItem


DATA_DIR = Path(__file__).resolve().parent / "data"
if str(DATA_DIR) not in sys.path:
    sys.path.insert(0, str(DATA_DIR))

from meta_template import create_meta_template  # noqa: E402


def observed_nutrient_keys(feeds: list[FeedItem]) -> set[str]:
    keys: set[str] = set()
    for feed in feeds:
        keys.update(feed.nutrition.keys())
    return keys


class OutputGenerator:
    """Generates JSON output files for the feed database."""

    def __init__(self, output_dir: Optional[Path] = None):
        self.output_dir = output_dir or OUTPUT_DIR
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.feeds: list[FeedItem] = []
        self._category_feeds: dict[str, list[FeedItem]] = {}
        self._feed_index: dict[str, FeedItem] = {}

    def add_feed(self, feed: FeedItem) -> None:
        """Add or replace a feed item."""

        if feed.source_url in self._feed_index:
            existing = self._feed_index[feed.source_url]
            self.feeds = [item for item in self.feeds if item.source_url != feed.source_url]
            if existing.category_id in self._category_feeds:
                self._category_feeds[existing.category_id] = [
                    item for item in self._category_feeds[existing.category_id] if item.source_url != feed.source_url
                ]

        self._feed_index[feed.source_url] = feed
        self.feeds.append(feed)
        self._category_feeds.setdefault(feed.category_id, []).append(feed)

    def load_existing_categories(self) -> None:
        """Load already saved category outputs for resume support."""

        for path in sorted(self.output_dir.glob("*.json")):
            if path.name == FINAL_DB_FILE.name:
                continue
            try:
                with open(path, "r", encoding="utf-8") as file:
                    data = json.load(file)
            except (OSError, json.JSONDecodeError):
                continue

            for raw_feed in data.get("feeds", []):
                self.add_feed(FeedItem.model_validate(raw_feed))

    def save_category(self, category_id: str) -> Path:
        """Save feeds for a specific category."""

        feeds = self._category_feeds.get(category_id, [])
        output_file = self.output_dir / f"{category_id}.json"
        data = {
            "category_id": category_id,
            "count": len(feeds),
            "feeds": [feed.model_dump(exclude_none=True) for feed in feeds],
        }

        with open(output_file, "w", encoding="utf-8") as file:
            json.dump(data, file, ensure_ascii=False, indent=2)

        return output_file

    def save_all_categories(self) -> list[Path]:
        """Save all category files."""

        return [self.save_category(category_id) for category_id in sorted(self._category_feeds)]

    def save_final_database(self, output_file: Optional[Path] = None, validate: bool = True) -> Path:
        """Save the final consolidated database."""

        output_file = output_file or (self.output_dir / FINAL_DB_FILE.name)

        meta = create_meta_template()
        observed_keys = observed_nutrient_keys(self.feeds)
        meta.nutrients = {
            key: value for key, value in meta.nutrients.items() if key in observed_keys
        }
        meta.total_feeds = len(self.feeds)
        meta.generated = datetime.now(timezone.utc).isoformat()

        database = FeedDatabase(meta=meta, feeds=self.feeds)
        data = database.model_dump(exclude_none=True)

        if validate and SCHEMA_FILE.exists():
            with open(SCHEMA_FILE, "r", encoding="utf-8") as file:
                schema = json.load(file)
            jsonschema.validate(data, schema)

        with open(output_file, "w", encoding="utf-8") as file:
            json.dump(data, file, ensure_ascii=False, indent=2)

        return output_file

    def get_stats(self) -> dict:
        """Return output statistics."""

        return {
            "total_feeds": len(self.feeds),
            "categories": len(self._category_feeds),
            "feeds_by_category": {
                category_id: len(feeds)
                for category_id, feeds in sorted(self._category_feeds.items())
            },
        }
