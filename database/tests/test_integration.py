"""Integration tests for the feed parser."""
import json

from models import FeedItem, TranslatableText
from output import OutputGenerator
from parser import parse_feed_card
from translator import Translator


LINSEED_CAKE_RU = "\u0416\u043c\u044b\u0445 \u043b\u044c\u043d\u044f\u043d\u043e\u0439"
CAKES_RU = "\u0416\u043c\u044b\u0445\u0438"


def test_full_parse_pipeline(sample_card_html, workspace_tmp_dir):
    """Test the complete parse pipeline from HTML to output."""

    data = parse_feed_card(sample_card_html, "https://vidkormov.narod.ru/card/n350.html")
    assert data["id"] == "n350"
    assert "crude_protein" in data["nutrition"]
    assert data["nutrition"]["crude_protein"]["value"] == 338.0
    assert data["nutrition"]["crude_protein"]["unit"] == "g"
    assert "metabolizable_energy" in data["nutrition"]
    assert "cattle" in data["nutrition"]["metabolizable_energy"]
    assert data["nutrition"]["metabolizable_energy"]["cattle"]["value"] == 11.7

    translator = Translator(use_api=False)
    name_en = translator.translate(data["name_ru"], category="feed_types")
    assert name_en == "Linseed cake"

    feed = FeedItem(
        id=data["id"],
        name=TranslatableText(ru=data["name_ru"], en=name_en),
        category_id="industrial_byproducts",
        subcategory=TranslatableText(ru=CAKES_RU, en="Cakes"),
        nutrition=data["nutrition"],
        source_url="https://vidkormov.narod.ru/card/n350.html",
    )

    output_dir = workspace_tmp_dir / "output"
    generator = OutputGenerator(output_dir)
    generator.add_feed(feed)
    db_path = generator.save_final_database()
    assert db_path.exists()

    with open(db_path, "r", encoding="utf-8") as file:
        db = json.load(file)

    assert db["meta"]["total_feeds"] == 1
    assert len(db["feeds"]) == 1
    assert db["feeds"][0]["name"]["ru"] == LINSEED_CAKE_RU
    assert db["feeds"][0]["name"]["en"] == "Linseed cake"
    assert db["feeds"][0]["nutrition"]["crude_protein"]["value"] == 338.0


def test_nutrient_translations():
    """Test that expected nutrients can be translated into stable keys."""

    translator = Translator(use_api=False)
    expected_nutrients = [
        ("\u041a\u043e\u0440\u043c\u043e\u0432\u044b\u0435 \u0435\u0434\u0438\u043d\u0438\u0446\u044b", "feed_units"),
        ("\u0421\u044b\u0440\u043e\u0439 \u043f\u0440\u043e\u0442\u0435\u0438\u043d", "crude_protein"),
        ("\u041f\u0435\u0440\u0435\u0432\u0430\u0440\u0438\u043c\u044b\u0439 \u043f\u0440\u043e\u0442\u0435\u0438\u043d", "digestible_protein"),
        ("\u041a\u0430\u043b\u044c\u0446\u0438\u0439", "calcium"),
        ("\u0424\u043e\u0441\u0444\u043e\u0440", "phosphorus"),
        ("\u041a\u0430\u0440\u043e\u0442\u0438\u043d", "carotene"),
        ("\u0412\u0438\u0442\u0430\u043c\u0438\u043d D", "vitamin_d"),
    ]

    for russian, expected_key in expected_nutrients:
        key = translator.get_nutrient_key(russian)
        assert key == expected_key, f"Expected {expected_key} for '{russian}', got {key}"
