"""Translation layer with static dictionary and API fallback."""
from __future__ import annotations

import json
import re
from typing import Optional

from config import STATIC_TRANSLATIONS_FILE, TRANSLATION_CACHE_FILE


class Translator:
    """Handles Russian to English translation."""

    NUTRIENT_KEY_MAP = {
        "Кормовые единицы": "feed_units",
        "Обменная энергия": "metabolizable_energy",
        "Сухое вещество": "dry_matter",
        "Сырой протеин": "crude_protein",
        "Переваримый протеин": "digestible_protein",
        "Лизин": "lysine",
        "Метионин+цистин": "methionine_cystine",
        "Метионин + цистин": "methionine_cystine",
        "Сырая клетчатка": "crude_fiber",
        "Сахара": "sugars",
        "Крахмал": "starch",
        "Сырой жир": "crude_fat",
        "Кальций": "calcium",
        "Фосфор": "phosphorus",
        "Магний": "magnesium",
        "Калий": "potassium",
        "Натрий": "sodium",
        "Сера": "sulfur",
        "Железо": "iron",
        "Медь": "copper",
        "Цинк": "zinc",
        "Марганец": "manganese",
        "Кобальт": "cobalt",
        "Йод": "iodine",
        "Каротин": "carotene",
        "Витамин D": "vitamin_d",
        "Витамин E": "vitamin_e",
    }

    ANIMAL_KEY_MAP = {
        "КРС": "cattle",
        "крс": "cattle",
        "крупного рогатого скота": "cattle",
        "Свиньи": "swine",
        "свиней": "swine",
        "Овцы": "sheep",
        "овец": "sheep",
        "Козы": "goats",
        "коз": "goats",
        "Птица": "poultry",
        "птицы": "poultry",
        "Куры": "chickens",
        "кур": "chickens",
        "Утки": "ducks",
        "Гуси": "geese",
        "Индейки": "turkeys",
        "Лошади": "horses",
        "лошадей": "horses",
        "Кролики": "rabbits",
        "кроликов": "rabbits",
        "Рыба": "fish",
        "рыб": "fish",
        "Пушные звери": "fur_animals",
    }

    def __init__(self, use_api: bool = True):
        self.use_api = use_api
        self.static_dict = self._load_static_translations()
        self.cache = self._load_cache()
        self._api_translator = None
        self._normalized_static = {
            category: {
                self._normalize_lookup_text(key): value
                for key, value in values.items()
            }
            for category, values in self.static_dict.items()
            if isinstance(values, dict)
        }
        self._normalized_nutrient_keys = {
            self._normalize_lookup_text(key): value
            for key, value in self.NUTRIENT_KEY_MAP.items()
        }
        self._normalized_animal_keys = {
            self._normalize_lookup_text(key): value
            for key, value in self.ANIMAL_KEY_MAP.items()
        }

    @staticmethod
    def _normalize_lookup_text(text: str) -> str:
        return re.sub(r"\s+", " ", text.replace("\xa0", " ").strip().lower()).replace("ё", "е")

    def _load_static_translations(self) -> dict:
        if STATIC_TRANSLATIONS_FILE.exists():
            with open(STATIC_TRANSLATIONS_FILE, "r", encoding="utf-8") as file:
                return json.load(file)
        return {"nutrients": {}, "animals": {}, "units": {}, "categories": {}, "regions": {}, "feed_types": {}}

    def _load_cache(self) -> dict:
        if TRANSLATION_CACHE_FILE.exists():
            with open(TRANSLATION_CACHE_FILE, "r", encoding="utf-8") as file:
                return json.load(file)
        return {}

    def _save_cache(self) -> None:
        TRANSLATION_CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(TRANSLATION_CACHE_FILE, "w", encoding="utf-8") as file:
            json.dump(self.cache, file, ensure_ascii=False, indent=2)

    def _get_api_translator(self):
        if self._api_translator is None and self.use_api:
            try:
                from deep_translator import GoogleTranslator

                self._api_translator = GoogleTranslator(source="ru", target="en")
            except ImportError:
                self._api_translator = False
        return self._api_translator

    def _lookup_static_translation(self, text: str, category: Optional[str] = None) -> Optional[str]:
        normalized = self._normalize_lookup_text(text)

        if category and category in self.static_dict:
            if text in self.static_dict[category]:
                return self.static_dict[category][text]
            return self._normalized_static.get(category, {}).get(normalized)

        for category_map in self.static_dict.values():
            if isinstance(category_map, dict) and text in category_map:
                return category_map[text]

        for category_map in self._normalized_static.values():
            if normalized in category_map:
                return category_map[normalized]

        return None

    def translate(self, text: str, category: Optional[str] = None) -> str:
        text = text.strip()
        if not text:
            return text

        static_translation = self._lookup_static_translation(text, category)
        if static_translation is not None:
            return static_translation

        if text in self.cache:
            return self.cache[text]

        translator = self._get_api_translator()
        if translator and translator is not False:
            try:
                result = translator.translate(text)
                self.cache[text] = result
                self._save_cache()
                return result
            except Exception:
                pass

        return text

    def get_nutrient_key(self, russian_name: str) -> Optional[str]:
        return self._normalized_nutrient_keys.get(self._normalize_lookup_text(russian_name))

    def get_animal_key(self, russian_name: str) -> Optional[str]:
        return self._normalized_animal_keys.get(self._normalize_lookup_text(russian_name))

    def normalize_unit(self, russian_unit: str) -> str:
        unit = russian_unit.strip()
        if not unit:
            return ""

        if unit in self.static_dict.get("units", {}):
            return self.static_dict["units"][unit]

        normalized = self._normalize_lookup_text(unit)
        normalized_map = self._normalized_static.get("units", {})
        if normalized in normalized_map:
            return normalized_map[normalized]

        if "тыс" in normalized and "ме" in normalized:
            return "thousand IU"
        if "млн" in normalized and "ме" in normalized:
            return "million IU"
        if "мдж" in normalized:
            return "MJ"
        if "мкг" in normalized:
            return "mcg"
        if normalized == "мг" or re.search(r"(?:^|\s)мг(?:$|\s)", normalized):
            return "mg"
        if normalized == "г" or re.search(r"(?:^|\s)г(?:$|\s)", normalized):
            return "g"
        if "ме" in normalized:
            return "IU"
        if "%" in unit:
            return "percent"
        if "к.ед" in normalized or "корм" in normalized:
            return "fu"

        return unit
