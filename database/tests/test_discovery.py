"""Tests for discovery module."""
from discovery import build_full_url, categorize_url, extract_links_from_html


def test_extract_links_from_index():
    """Test extracting category links from an index page."""

    html = """
    <html><body>
    <ul>
        <li><a href="spis/m110000.html">Зеленые корма</a></li>
        <li><a href="spis/m120000.html">Грубые корма</a></li>
        <li><a href="list/o150000.html">Отходы промышленности</a></li>
    </ul>
    </body></html>
    """
    links = extract_links_from_html(html, "https://vidkormov.narod.ru/index.html")
    assert len(links) == 3
    assert "https://vidkormov.narod.ru/spis/m110000.html" in links
    assert "https://vidkormov.narod.ru/list/o150000.html" in links


def test_extract_card_links():
    """Test extracting card links from a listing page."""

    html = """
    <html><body>
    <ul>
        <li><a href="../card/n350.html">Жмых льняной</a></li>
        <li><a href="../card/n351.html">Жмых подсолнечный</a></li>
    </ul>
    </body></html>
    """
    links = extract_links_from_html(html, "https://vidkormov.narod.ru/grup/n27.html")
    assert "https://vidkormov.narod.ru/card/n350.html" in links
    assert "https://vidkormov.narod.ru/card/n351.html" in links


def test_categorize_url():
    """Test URL categorization."""

    assert categorize_url("https://vidkormov.narod.ru/index.html") == "index"
    assert categorize_url("https://vidkormov.narod.ru/spis/m110000.html") == "category"
    assert categorize_url("https://vidkormov.narod.ru/list/o150000.html") == "category"
    assert categorize_url("https://vidkormov.narod.ru/vid2/s1.html") == "region"
    assert categorize_url("https://vidkormov.narod.ru/typ2/s1.html") == "type"
    assert categorize_url("https://vidkormov.narod.ru/grup/n27.html") == "group"
    assert categorize_url("https://vidkormov.narod.ru/card/n350.html") == "card"
    assert categorize_url("https://vidkormov.narod.ru/komb/m190100.html") == "mixed_section"
    assert categorize_url("https://vidkormov.narod.ru/komb1/s190110.html") == "mixed_subsection"
    assert categorize_url("https://vidkormov.narod.ru/kkorm/n12.html") == "card"


def test_build_full_url():
    """Test building a full URL from a relative path."""

    base = "https://vidkormov.narod.ru/grup/n27.html"
    assert build_full_url("../card/n350.html", base) == "https://vidkormov.narod.ru/card/n350.html"
    assert build_full_url("n28.html", base) == "https://vidkormov.narod.ru/grup/n28.html"
