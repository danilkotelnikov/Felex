"""Tests for the content parser."""
from parser import extract_value_and_unit, parse_feed_card, parse_nutrient_table


def test_extract_value_and_unit_simple():
    """Test extracting a simple value with unit."""

    value, unit = extract_value_and_unit("338 г")
    assert value == 338.0
    assert unit == "g"


def test_extract_value_and_unit_decimal():
    """Test extracting a decimal value."""

    value, unit = extract_value_and_unit("11.7 МДж")
    assert value == 11.7
    assert unit == "MJ"


def test_extract_value_and_unit_no_space():
    """Test extracting a value without a space before the unit."""

    value, unit = extract_value_and_unit("3.4г")
    assert value == 3.4
    assert unit == "g"


def test_extract_value_and_unit_comma_decimal():
    """Test Russian comma decimals."""

    value, unit = extract_value_and_unit("11,7 МДж")
    assert value == 11.7
    assert unit == "MJ"


def test_parse_nutrient_table():
    """Test parsing a nutrient table."""

    html = """
    <table>
        <tr><td>Сухое вещество</td><td>900 г</td></tr>
        <tr><td>Сырой протеин</td><td>338 г</td></tr>
        <tr><td>Кальций</td><td>3.4 г</td></tr>
        <tr><td>Обменная энергия КРС</td><td>11.7 МДж</td></tr>
        <tr><td>Обменная энергия свиней</td><td>13.73 МДж</td></tr>
    </table>
    """
    nutrients = parse_nutrient_table(html)

    assert "dry_matter" in nutrients
    assert nutrients["dry_matter"]["value"] == 900.0
    assert nutrients["dry_matter"]["unit"] == "g"
    assert "crude_protein" in nutrients
    assert nutrients["crude_protein"]["value"] == 338.0
    assert "calcium" in nutrients
    assert nutrients["calcium"]["value"] == 3.4
    assert "metabolizable_energy" in nutrients
    assert "cattle" in nutrients["metabolizable_energy"]
    assert nutrients["metabolizable_energy"]["cattle"]["value"] == 11.7


def test_parse_nutrient_table_units_in_label():
    """Test parsing the live-site pattern where units live in the label column."""

    html = """
    <table>
        <tr><td>Сухое вещество, г</td><td>900</td></tr>
        <tr><td>Сырой протеин, г</td><td>338</td></tr>
        <tr><td>Обменная энергия (КРС), МДж</td><td>11,7</td></tr>
    </table>
    """
    nutrients = parse_nutrient_table(html)

    assert nutrients["dry_matter"]["unit"] == "g"
    assert nutrients["crude_protein"]["unit"] == "g"
    assert nutrients["metabolizable_energy"]["cattle"]["unit"] == "MJ"


def test_parse_feed_card_full():
    """Test parsing a complete feed card page."""

    html = """
    <html><body>
    <h1>Жмых льняной</h1>
    <p>Категория: Жмыхи и шроты</p>
    <table>
        <tr><td>Кормовые единицы</td><td>1.27</td></tr>
        <tr><td>Сухое вещество</td><td>900 г</td></tr>
        <tr><td>Сырой протеин</td><td>338 г</td></tr>
    </table>
    </body></html>
    """
    result = parse_feed_card(html, "https://vidkormov.narod.ru/card/n350.html")

    assert result["id"] == "n350"
    assert result["name_ru"] == "Жмых льняной"
    assert "feed_units" in result["nutrition"]
    assert result["nutrition"]["feed_units"]["value"] == 1.27


def test_parse_feed_card_uses_fallback_name_for_generic_pages():
    """Test that generic titles fall back to the discovery-time feed name."""

    html = """
    <html><head><title>Состав - Корма России</title></head><body>
    <h2>Состав</h2>
    <table>
        <tr><td>Кормовые единицы</td><td>0,85</td></tr>
    </table>
    </body></html>
    """
    result = parse_feed_card(
        html,
        "https://vidkormov.narod.ru/kkorm/n12.html",
        fallback_name="Рецепты комбикормов для ремонтных телок",
    )
    assert result["name_ru"] == "Рецепты комбикормов для ремонтных телок"
