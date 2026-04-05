"""Tests for Pydantic models."""
import pytest
from pydantic import ValidationError


def test_translatable_text_requires_ru():
    """Test that TranslatableText requires ru field."""

    from models import TranslatableText

    with pytest.raises(ValidationError):
        TranslatableText(en="English only")


def test_translatable_text_en_optional():
    """Test that en field is optional with default."""

    from models import TranslatableText

    text = TranslatableText(ru="Русский")
    assert text.ru == "Русский"
    assert text.en == ""


def test_nutrient_value_requires_both_fields():
    """Test NutrientValue requires value and unit."""

    from models import NutrientValue

    nutrient = NutrientValue(value=100.5, unit="g")
    assert nutrient.value == 100.5
    assert nutrient.unit == "g"


def test_feed_item_minimal():
    """Test creating a minimal FeedItem."""

    from models import FeedItem, TranslatableText

    feed = FeedItem(
        id="n1",
        name=TranslatableText(ru="Корм", en="Feed"),
        category_id="green_feeds",
        subcategory=TranslatableText(ru="Травы", en="Grasses"),
        source_url="https://example.com/card/n1.html",
    )
    assert feed.id == "n1"
    assert feed.nutrition == {}
    assert feed.parse_errors == []
