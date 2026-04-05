"""Tests for the output generator."""
import json

import pytest

from models import FeedItem, TranslatableText
from output import OutputGenerator


@pytest.fixture
def temp_output_dir(workspace_tmp_dir):
    """Create a temporary output directory."""

    return workspace_tmp_dir / "output"


def test_add_feed(temp_output_dir):
    """Test adding a feed item."""

    generator = OutputGenerator(temp_output_dir)
    feed = FeedItem(
        id="n350",
        name=TranslatableText(ru="Жмых льняной", en="Linseed cake"),
        category_id="industrial_byproducts",
        subcategory=TranslatableText(ru="Жмыхи", en="Cakes"),
        nutrition={"crude_protein": {"value": 338, "unit": "g"}},
        source_url="https://example.com/card/n350.html",
    )

    generator.add_feed(feed)
    assert len(generator.feeds) == 1
    assert generator.feeds[0].id == "n350"


def test_save_category_file(temp_output_dir):
    """Test saving a category JSON file."""

    generator = OutputGenerator(temp_output_dir)
    feed = FeedItem(
        id="n350",
        name=TranslatableText(ru="Жмых льняной", en="Linseed cake"),
        category_id="industrial_byproducts",
        subcategory=TranslatableText(ru="Жмыхи", en="Cakes"),
        nutrition={},
        source_url="https://example.com/card/n350.html",
    )

    generator.add_feed(feed)
    generator.save_category("industrial_byproducts")

    output_file = temp_output_dir / "industrial_byproducts.json"
    assert output_file.exists()

    with open(output_file, "r", encoding="utf-8") as file:
        data = json.load(file)
    assert len(data["feeds"]) == 1
    assert data["feeds"][0]["id"] == "n350"


def test_save_final_database(temp_output_dir):
    """Test saving the final consolidated database."""

    generator = OutputGenerator(temp_output_dir)
    feed1 = FeedItem(
        id="n350",
        name=TranslatableText(ru="Жмых льняной", en="Linseed cake"),
        category_id="industrial_byproducts",
        subcategory=TranslatableText(ru="Жмыхи", en="Cakes"),
        nutrition={},
        source_url="https://example.com/card/n350.html",
    )
    feed2 = FeedItem(
        id="n1",
        name=TranslatableText(ru="Трава", en="Grass"),
        category_id="green_feeds",
        subcategory=TranslatableText(ru="Травы", en="Grasses"),
        nutrition={},
        source_url="https://example.com/card/n1.html",
    )

    generator.add_feed(feed1)
    generator.add_feed(feed2)
    generator.save_final_database()

    db_file = temp_output_dir / "feeds_database.json"
    assert db_file.exists()

    with open(db_file, "r", encoding="utf-8") as file:
        data = json.load(file)

    assert "meta" in data
    assert "feeds" in data
    assert len(data["feeds"]) == 2
    assert data["meta"]["total_feeds"] == 2
