"""Tests for translator module."""
from translator import Translator


CALCIUM_RU = "\u041a\u0430\u043b\u044c\u0446\u0438\u0439"
CATTLE_RU = "\u041a\u0420\u0421"
UNKNOWN_RU = "\u041d\u0435\u0438\u0437\u0432\u0435\u0441\u0442\u043d\u044b\u0439 \u0442\u0435\u0440\u043c\u0438\u043d"
CRUDE_PROTEIN_RU = "\u0421\u044b\u0440\u043e\u0439 \u043f\u0440\u043e\u0442\u0435\u0438\u043d"
VITAMIN_D_RU = "\u0412\u0438\u0442\u0430\u043c\u0438\u043d D"
SWINE_GENITIVE_RU = "\u0441\u0432\u0438\u043d\u0435\u0439"


def test_translate_known_nutrient():
    """Test translation of known nutrient from the static dictionary."""

    translator = Translator()
    assert translator.translate(CALCIUM_RU, category="nutrients") == "Calcium"


def test_translate_known_animal():
    """Test translation of known animal type."""

    translator = Translator()
    assert translator.translate(CATTLE_RU, category="animals") == "Cattle"


def test_translate_unknown_returns_original():
    """Test that unknown text returns the original when API is disabled."""

    translator = Translator(use_api=False)
    assert translator.translate(UNKNOWN_RU, category="nutrients") == UNKNOWN_RU


def test_normalize_russian_text():
    """Test text normalization."""

    translator = Translator()
    assert translator.translate(f"  {CALCIUM_RU}  ", category="nutrients") == "Calcium"


def test_get_nutrient_key():
    """Test converting Russian nutrient names to snake_case keys."""

    translator = Translator()
    assert translator.get_nutrient_key(CRUDE_PROTEIN_RU) == "crude_protein"
    assert translator.get_nutrient_key(VITAMIN_D_RU) == "vitamin_d"
    assert translator.get_nutrient_key("Unknown") is None


def test_get_animal_key():
    """Test converting Russian animal names to keys."""

    translator = Translator()
    assert translator.get_animal_key(CATTLE_RU) == "cattle"
    assert translator.get_animal_key("\u043a\u0440\u0441") == "cattle"
    assert translator.get_animal_key(SWINE_GENITIVE_RU) == "swine"


def test_normalize_unit_variants():
    """Test normalization of common unit variants."""

    translator = Translator()
    assert translator.normalize_unit("\u043c\u0433") == "mg"
    assert translator.normalize_unit("\u041c\u0414\u0436") == "MJ"
    assert translator.normalize_unit("\u0442\u044b\u0441. \u041c\u0415") == "thousand IU"
