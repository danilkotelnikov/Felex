"""HTML content parser for feed data extraction."""
from __future__ import annotations

import re
from typing import Optional

from bs4 import BeautifulSoup

from discovery import extract_id_from_url
from translator import Translator


_translator = Translator(use_api=False)
_GENERIC_NAMES = {"", "Состав", "Корма России", "Типы кормов", "Химический состав"}


def extract_value_and_unit(text: str) -> tuple[Optional[float], str]:
    """Extract numeric value and optional unit from text."""

    cleaned = text.replace("\xa0", " ").strip().replace(",", ".")
    if not cleaned:
        return None, ""

    match = re.match(r"([-+]?\d[\d\s]*(?:\.\d+)?)\s*(.*)$", cleaned)
    if not match:
        return None, cleaned

    try:
        value = float(match.group(1).replace(" ", ""))
    except ValueError:
        return None, cleaned

    unit_raw = match.group(2).strip()
    unit = _translator.normalize_unit(unit_raw) if unit_raw else ""
    return value, unit


def extract_animal_from_label(label: str) -> Optional[str]:
    """Extract animal key from nutrient label."""

    normalized = label.replace("\xa0", " ")
    patterns = [
        (r"\((?:КРС|крс|крупного рогатого скота)\)|\b(?:КРС|крс|крупного рогатого скота)\b", "cattle"),
        (r"\((?:свиньи|свиней)\)|\b(?:свиньи|свиней)\b", "swine"),
        (r"\((?:овцы|овец)\)|\b(?:овцы|овец)\b", "sheep"),
        (r"\((?:козы|коз)\)|\b(?:козы|коз)\b", "goats"),
        (r"\((?:птица|птицы)\)|\b(?:птица|птицы)\b", "poultry"),
        (r"\((?:куры|кур)\)|\b(?:куры|кур)\b", "chickens"),
        (r"\((?:утки)\)|\b(?:утки)\b", "ducks"),
        (r"\((?:гуси)\)|\b(?:гуси)\b", "geese"),
        (r"\((?:индейки)\)|\b(?:индейки)\b", "turkeys"),
        (r"\((?:лошади|лошадей)\)|\b(?:лошади|лошадей)\b", "horses"),
        (r"\((?:кролики|кроликов)\)|\b(?:кролики|кроликов)\b", "rabbits"),
        (r"\((?:рыба|рыб)\)|\b(?:рыба|рыб)\b", "fish"),
    ]

    for pattern, animal_key in patterns:
        if re.search(pattern, normalized, re.IGNORECASE):
            return animal_key

    return None


def extract_unit_from_label(label: str) -> str:
    """Extract a unit encoded in the nutrient label."""

    if "," not in label:
        return ""

    _, possible_unit = label.rsplit(",", 1)
    normalized = _translator.normalize_unit(possible_unit.strip())
    return "" if normalized == possible_unit.strip() and not possible_unit.strip().startswith("%") else normalized


def extract_base_nutrient(label: str) -> str:
    """Remove animal qualifiers and unit suffix from a nutrient label."""

    result = label.replace("\xa0", " ").strip()
    result = re.sub(r"\((?:[^)]*)\)", "", result).strip()
    result = re.sub(
        r"\s+(?:КРС|крс|крупного рогатого скота|свиньи|свиней|овцы|овец|козы|коз|"
        r"птица|птицы|куры|кур|утки|гуси|индейки|лошади|лошадей|кролики|кроликов|"
        r"рыба|рыб)$",
        "",
        result,
        flags=re.IGNORECASE,
    ).strip()
    if "," in result:
        result = result.rsplit(",", 1)[0].strip()
    return re.sub(r"\s+", " ", result)


def parse_nutrient_table(html: str, return_errors: bool = False):
    """Parse nutrient rows from HTML tables."""

    soup = BeautifulSoup(html, "lxml")
    nutrients: dict[str, dict] = {}
    parse_errors: list[dict] = []

    for table in soup.find_all("table"):
        rows = table.find_all("tr")
        for row in rows:
            cells = row.find_all(["td", "th"])
            if len(cells) < 2:
                continue

            label = cells[0].get_text(" ", strip=True)
            value_text = cells[1].get_text(" ", strip=True)
            if not label or not value_text:
                continue

            if label.lower() in {"показатель", "параметр", "название", "корма"}:
                continue

            value, unit = extract_value_and_unit(value_text)
            if value is None:
                continue

            if not unit:
                unit = extract_unit_from_label(label)

            animal_key = extract_animal_from_label(label)
            base_label = extract_base_nutrient(label)
            nutrient_key = _translator.get_nutrient_key(base_label) or _translator.get_nutrient_key(label)
            if not nutrient_key:
                continue

            if animal_key:
                nutrients.setdefault(nutrient_key, {})
                nutrients[nutrient_key][animal_key] = {"value": value, "unit": unit}
            else:
                if nutrient_key not in nutrients:
                    nutrients[nutrient_key] = {"value": value, "unit": unit}
                elif "value" not in nutrients[nutrient_key]:
                    nutrients[nutrient_key]["universal"] = {"value": value, "unit": unit}

    if return_errors:
        return nutrients, parse_errors
    return nutrients


def _clean_title_text(title_text: str) -> str:
    title = title_text.strip()
    if " - " in title:
        title = title.split(" - ", 1)[0].strip()
    title = re.sub(r"\((?:химический состав|состав)\)", "", title, flags=re.IGNORECASE).strip()
    return re.sub(r"\s+", " ", title)


def _extract_feed_name(soup: BeautifulSoup, fallback_name: str = "") -> str:
    title = soup.find("title")
    if title:
        cleaned_title = _clean_title_text(title.get_text(" ", strip=True))
        if cleaned_title not in _GENERIC_NAMES:
            return cleaned_title

    for tag in ("h1", "h2", "h3"):
        heading = soup.find(tag)
        if heading:
            candidate = re.sub(r"\s+", " ", heading.get_text(" ", strip=True))
            if candidate not in _GENERIC_NAMES:
                return candidate

    return fallback_name.strip()


def parse_feed_card(html: str, url: str, fallback_name: str = "") -> dict:
    """Parse a complete feed card page."""

    soup = BeautifulSoup(html, "lxml")
    feed_id = extract_id_from_url(url)
    name_ru = _extract_feed_name(soup, fallback_name=fallback_name)
    nutrition, parse_errors = parse_nutrient_table(html, return_errors=True)

    return {
        "id": feed_id,
        "name_ru": name_ru,
        "nutrition": nutrition,
        "source_url": url,
        "parse_errors": parse_errors,
    }
