from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
SOURCE_DIR = ROOT / "database" / "output"
APP_DIR = SOURCE_DIR / "app"
RUNTIME_DIR = APP_DIR / "runtime"
FRONTEND_GENERATED_DIR = ROOT / "frontend" / "src" / "generated"
CONSOLIDATED_PATH = SOURCE_DIR / "feeds_database.json"
STATIC_TRANSLATIONS_PATH = ROOT / "database" / "data" / "static_translations.json"
CATEGORY_FILES = [
    "animal_feeds.json",
    "green_feeds.json",
    "rough_feeds.json",
    "succulent_feeds.json",
    "concentrated_feeds.json",
    "industrial_byproducts.json",
    "mineral_supplements.json",
    "mixed_feeds.json",
    "nitrogen_compounds.json",
]

RUNTIME_CATEGORY_LABELS: dict[str, dict[str, str]] = {
    "grain": {"ru": "Зерновые", "en": "Grains"},
    "concentrate": {"ru": "Концентраты", "en": "Concentrates"},
    "oilseed_meal": {"ru": "Жмыхи и шроты", "en": "Oilseed meals"},
    "protein": {"ru": "Белковые корма", "en": "Protein feeds"},
    "roughage": {"ru": "Грубые корма", "en": "Roughage"},
    "silage": {"ru": "Силос и сенаж", "en": "Silage and haylage"},
    "succulent": {"ru": "Сочные корма", "en": "Succulent feeds"},
    "animal_origin": {"ru": "Корма животного происхождения", "en": "Animal-origin feeds"},
    "mineral": {"ru": "Минеральные добавки", "en": "Mineral supplements"},
    "premix": {"ru": "Премиксы", "en": "Premixes"},
    "additive": {"ru": "Кормовые добавки", "en": "Feed additives"},
    "other": {"ru": "Прочие", "en": "Other"},
}


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(handle)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        json.dump(payload, handle, ensure_ascii=False, indent=2)
        handle.write("\n")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False))
            handle.write("\n")


def normalized_text(value: str | None) -> str:
    return (value or "").strip().lower()


def normalize_lookup_text(value: str | None) -> str:
    return " ".join(normalized_text((value or "").replace("\xa0", " ")).split()).replace("ё", "е")


def normalized_unit(unit: str | None) -> str:
    return normalized_text(unit)


def has_any(haystack: str, needles: list[str]) -> bool:
    return any(needle in haystack for needle in needles)


def is_single_value(value: Any) -> bool:
    return isinstance(value, dict) and "value" in value


def single_value(feed: dict[str, Any], key: str) -> dict[str, Any] | None:
    raw = feed.get("nutrition", {}).get(key)
    if is_single_value(raw):
        return raw
    return None


def animal_value(feed: dict[str, Any], key: str, aliases: list[str]) -> dict[str, Any] | None:
    raw = feed.get("nutrition", {}).get(key)
    if is_single_value(raw):
        return raw
    if not isinstance(raw, dict):
        return None
    for alias in aliases:
        value = raw.get(alias)
        if is_single_value(value):
            return value
    universal = raw.get("universal")
    return universal if is_single_value(universal) else None


def convert_dry_matter_pct(value: dict[str, Any] | None) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "g":
        return amount / 10.0
    if unit == "%":
        return amount
    if unit == "kg":
        return amount * 100.0
    return None


def convert_energy_mj(value: dict[str, Any] | None) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "mj":
        return amount
    return None


def convert_grams_per_kg(value: dict[str, Any] | None) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "g":
        return amount
    if unit == "mg":
        return amount / 1000.0
    if unit == "kg":
        return amount * 1000.0
    return None


def convert_mg_per_kg(value: dict[str, Any] | None) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "mg":
        return amount
    if unit == "g":
        return amount * 1000.0
    if unit in {"mcg", "ug"}:
        return amount / 1000.0
    return None


def convert_trace_mineral_mg(value: dict[str, Any] | None, nutrient_key: str) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "mg":
        return amount
    if unit == "g" and nutrient_key == "iron":
        return amount
    if unit == "g":
        return amount * 1000.0
    if unit in {"mcg", "ug"}:
        return amount / 1000.0
    return None


def convert_vitamin_iu(value: dict[str, Any] | None) -> float | None:
    if not value:
        return None
    unit = normalized_unit(value.get("unit"))
    amount = value.get("value")
    if unit == "iu":
        return amount
    if unit == "thousand iu":
        return amount * 1000.0
    if unit == "million iu":
        return amount * 1_000_000.0
    return None


def uses_monogastric_proxy(category: str) -> bool:
    return category in {"grain", "concentrate", "oilseed_meal", "protein", "animal_origin"}


def load_static_translations() -> dict[str, dict[str, str]]:
    if not STATIC_TRANSLATIONS_PATH.exists():
        return {}
    payload = read_json(STATIC_TRANSLATIONS_PATH)
    return {
        category: values
        for category, values in payload.items()
        if isinstance(values, dict)
    }


def lookup_static_translation(
    static_translations: dict[str, dict[str, str]],
    text: str,
    categories: list[str],
) -> str | None:
    normalized = normalize_lookup_text(text)
    for category in categories:
        mapping = static_translations.get(category, {})
        if text in mapping:
            return mapping[text]
        for key, value in mapping.items():
            if normalize_lookup_text(key) == normalized:
                return value
    return None


def normalized_name_en(
    raw_feed: dict[str, Any],
    static_translations: dict[str, dict[str, str]],
) -> str | None:
    name = raw_feed.get("name", {})
    ru = (name.get("ru") or "").strip()
    en = (name.get("en") or "").strip()
    if not en or normalized_text(en) == normalized_text(ru):
        translated = lookup_static_translation(static_translations, ru, ["feed_types", "categories"])
        return translated if translated and normalize_lookup_text(translated) != normalize_lookup_text(ru) else None
    return en


def mapped_category(raw_feed: dict[str, Any]) -> str:
    category_id = raw_feed.get("category_id") or ""
    subcategory = (raw_feed.get("subcategory", {}) or {}).get("ru", "")
    name = (raw_feed.get("name", {}) or {}).get("ru", "")
    search = f"{category_id} {subcategory} {name} {(raw_feed.get('name', {}) or {}).get('en', '')}".lower()

    looks_like_premix = has_any(
        search, ["premix", "премикс", "бвмд", "бмвд", "комбикорм-концентрат"]
    )
    looks_like_mineral = has_any(
        search,
        [
            "chalk",
            "limestone",
            "salt",
            "phosphate",
            "shell",
            "мел",
            "соль",
            "фосфат",
            "известняк",
            "ракуш",
        ],
    )
    looks_like_oilseed = has_any(
        search,
        [
            "meal",
            "cake",
            "шрот",
            "жмых",
            "соев",
            "подсолнеч",
            "рапс",
            "хлопк",
        ],
    )
    looks_like_protein = has_any(
        search, ["protein", "белк", "горох", "люпин", "бобы", "дрожж", "yeast"]
    )
    looks_like_grain = has_any(
        search,
        [
            "grain",
            "зерно",
            "пшениц",
            "ячмен",
            "кукуруз",
            "овес",
            "овёс",
            "рож",
            "тритикале",
            "bran",
            "отруб",
        ],
    )
    looks_like_silage = has_any(search, ["silage", "haylage", "силос", "сенаж"])

    if category_id == "rough_feeds":
        return "roughage"
    if category_id in {"green_feeds", "succulent_feeds"}:
        return "silage" if looks_like_silage else "succulent"
    if category_id == "animal_feeds":
        return "animal_origin"
    if category_id == "nitrogen_compounds":
        return "additive"
    if category_id == "mineral_supplements":
        return "premix" if looks_like_premix else "mineral"
    if category_id == "mixed_feeds":
        return "premix" if looks_like_premix else "concentrate"
    if category_id == "industrial_byproducts":
        if looks_like_oilseed:
            return "oilseed_meal"
        if looks_like_protein:
            return "protein"
        return "concentrate"
    if category_id == "concentrated_feeds":
        if looks_like_premix:
            return "premix"
        if looks_like_mineral:
            return "mineral"
        if looks_like_oilseed:
            return "oilseed_meal"
        if looks_like_protein:
            return "protein"
        if looks_like_grain:
            return "grain"
        return "concentrate"
    return "other"


def deterministic_id(source_key: str) -> int:
    digest = hashlib.sha1(source_key.encode("utf-8")).hexdigest()[:8]
    return int(digest, 16) % 2_000_000_000 + 1


def build_notes(raw_feed: dict[str, Any]) -> str:
    lines = [
        "Seeded from normalized feed database source family",
        f"Original category: {raw_feed.get('category_id', '')}",
    ]
    subcategory = (raw_feed.get("subcategory", {}) or {}).get("ru", "").strip()
    if subcategory:
        lines.append(f"Original subcategory: {subcategory}")
    parse_errors = raw_feed.get("parse_errors") or []
    if parse_errors:
        fields = ", ".join(
            item.get("field", "")
            for item in parse_errors[:3]
            if item.get("field")
        )
        suffix = f" ({fields})" if fields else ""
        lines.append(f"Source parse warnings: {len(parse_errors)}{suffix}")
    return "\n".join(lines)


def resolve_subcategory_en(
    raw_feed: dict[str, Any],
    static_translations: dict[str, dict[str, str]],
) -> str | None:
    subcategory = raw_feed.get("subcategory", {}) or {}
    ru = (subcategory.get("ru") or "").strip()
    en = (subcategory.get("en") or "").strip()
    if en and normalize_lookup_text(en) != normalize_lookup_text(ru):
        return en
    translated = lookup_static_translation(static_translations, ru, ["feed_types", "categories"])
    return translated if translated and normalize_lookup_text(translated) != normalize_lookup_text(ru) else None


def build_feed_record(
    raw_feed: dict[str, Any],
    static_translations: dict[str, dict[str, str]],
) -> dict[str, Any]:
    dm_pct = convert_dry_matter_pct(single_value(raw_feed, "dry_matter"))
    dm_share = dm_pct / 100.0 if dm_pct and dm_pct > 0 else None
    category = mapped_category(raw_feed)

    def energy_oe(aliases: list[str]) -> float | None:
        value = convert_energy_mj(animal_value(raw_feed, "metabolizable_energy", aliases))
        if value is None:
            return None
        if dm_share:
            return value / dm_share
        return value

    def digestible_protein(aliases: list[str]) -> float | None:
        return convert_grams_per_kg(animal_value(raw_feed, "digestible_protein", aliases))

    name = raw_feed.get("name", {})
    source_key = f"seed:normalized-db:{raw_feed.get('id', '')}"
    parse_errors = raw_feed.get("parse_errors") or []
    name_en = normalized_name_en(raw_feed, static_translations)
    source_subcategory_en = resolve_subcategory_en(raw_feed, static_translations)
    translation_status = "ready" if name_en else "source_only"
    energy_oe_poultry = energy_oe(["poultry", "chickens", "ducks", "geese", "turkeys"])
    if energy_oe_poultry is None and uses_monogastric_proxy(category):
        energy_oe_poultry = energy_oe(["swine"])
    dig_protein_poultry = digestible_protein(["poultry", "chickens"])
    if dig_protein_poultry is None and uses_monogastric_proxy(category):
        dig_protein_poultry = digestible_protein(["swine"])

    return {
        "id": deterministic_id(source_key),
        "source_id": source_key,
        "source_url": raw_feed.get("source_url"),
        "name_ru": (name.get("ru") or "").strip(),
        "name_en": name_en,
        "category": category,
        "subcategory": ((raw_feed.get("subcategory", {}) or {}).get("ru") or "").strip() or None,
        "source_category_id": raw_feed.get("category_id"),
        "source_subcategory_en": source_subcategory_en,
        "source_nutrition": raw_feed.get("nutrition") or {},
        "dry_matter": dm_pct,
        "energy_oe_cattle": energy_oe(["cattle"]),
        "energy_oe_pig": energy_oe(["swine"]),
        "energy_oe_poultry": energy_oe_poultry,
        "koe": (single_value(raw_feed, "feed_units") or {}).get("value"),
        "crude_protein": convert_grams_per_kg(single_value(raw_feed, "crude_protein")),
        "dig_protein_cattle": digestible_protein(["cattle"]),
        "dig_protein_pig": digestible_protein(["swine"]),
        "dig_protein_poultry": dig_protein_poultry,
        "lysine": convert_grams_per_kg(single_value(raw_feed, "lysine")),
        "methionine_cystine": convert_grams_per_kg(single_value(raw_feed, "methionine_cystine")),
        "crude_fat": convert_grams_per_kg(single_value(raw_feed, "crude_fat")),
        "crude_fiber": convert_grams_per_kg(single_value(raw_feed, "crude_fiber")),
        "starch": convert_grams_per_kg(single_value(raw_feed, "starch")),
        "sugar": convert_grams_per_kg(single_value(raw_feed, "sugars")),
        "calcium": convert_grams_per_kg(single_value(raw_feed, "calcium")),
        "phosphorus": convert_grams_per_kg(single_value(raw_feed, "phosphorus")),
        "magnesium": convert_grams_per_kg(single_value(raw_feed, "magnesium")),
        "potassium": convert_grams_per_kg(single_value(raw_feed, "potassium")),
        "sodium": convert_grams_per_kg(single_value(raw_feed, "sodium")),
        "sulfur": convert_grams_per_kg(single_value(raw_feed, "sulfur")),
        "iron": convert_trace_mineral_mg(single_value(raw_feed, "iron"), "iron"),
        "copper": convert_trace_mineral_mg(single_value(raw_feed, "copper"), "copper"),
        "zinc": convert_trace_mineral_mg(single_value(raw_feed, "zinc"), "zinc"),
        "manganese": convert_trace_mineral_mg(single_value(raw_feed, "manganese"), "manganese"),
        "cobalt": convert_trace_mineral_mg(single_value(raw_feed, "cobalt"), "cobalt"),
        "iodine": convert_trace_mineral_mg(single_value(raw_feed, "iodine"), "iodine"),
        "carotene": convert_mg_per_kg(single_value(raw_feed, "carotene")),
        "vit_d3": convert_vitamin_iu(single_value(raw_feed, "vitamin_d")),
        "vit_e": convert_mg_per_kg(single_value(raw_feed, "vitamin_e")),
        "moisture": 100.0 - dm_pct if dm_pct is not None else None,
        "feed_conversion": None,
        "palatability": None,
        "max_inclusion_cattle": None,
        "max_inclusion_pig": None,
        "max_inclusion_poultry": None,
        "price_per_ton": None,
        "price_updated_at": None,
        "region": raw_feed.get("region_id"),
        "is_custom": False,
        "verified": len(parse_errors) == 0,
        "notes": build_notes(raw_feed),
        "source_kind": "normalized",
        "translation_status": translation_status,
        "profile_status": None,
        "profile_sections": None,
        "created_at": None,
        "updated_at": None,
    }


def build_catalog_record(feed_record: dict[str, Any]) -> dict[str, Any]:
    keys = [
        "id",
        "source_id",
        "name_ru",
        "name_en",
        "category",
        "subcategory",
        "source_category_id",
        "source_subcategory_en",
        "dry_matter",
        "energy_oe_cattle",
        "energy_oe_pig",
        "energy_oe_poultry",
        "crude_protein",
        "dig_protein_cattle",
        "dig_protein_pig",
        "dig_protein_poultry",
        "lysine",
        "methionine_cystine",
        "crude_fiber",
        "starch",
        "sugar",
        "calcium",
        "phosphorus",
        "carotene",
        "vit_d3",
        "vit_e",
        "price_per_ton",
        "price_updated_at",
        "region",
        "verified",
        "source_kind",
        "translation_status",
    ]
    record: dict[str, Any] = {}
    for key in keys:
        value = feed_record.get(key)
        if value is None:
            continue
        if isinstance(value, str) and not value.strip():
            continue
        record[key] = value
    return record


def load_source_family() -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, dict[str, Any]]]:
    consolidated = read_json(CONSOLIDATED_PATH) if CONSOLIDATED_PATH.exists() else {}
    shard_payloads: dict[str, dict[str, Any]] = {}
    for file_name in CATEGORY_FILES:
        path = SOURCE_DIR / file_name
        if path.exists():
            shard_payloads[file_name] = read_json(path)

    if shard_payloads:
        feeds: list[dict[str, Any]] = []
        for payload in shard_payloads.values():
            feeds.extend(payload.get("feeds", []))
    elif consolidated:
        feeds = consolidated.get("feeds", [])
    else:
        raise FileNotFoundError("No source-family files found in database/output")

    return consolidated, feeds, shard_payloads


def collect_subcategory_localization(
    raw_feeds: list[dict[str, Any]],
    static_translations: dict[str, dict[str, str]],
) -> dict[str, dict[str, str]]:
    localized: dict[str, dict[str, str]] = {}
    for raw_feed in raw_feeds:
        subcategory = raw_feed.get("subcategory") or {}
        ru = (subcategory.get("ru") or "").strip()
        en = (subcategory.get("en") or "").strip()
        if not ru:
            continue

        existing = localized.get(ru)
        if existing and existing.get("en"):
            continue

        translated_en = en or lookup_static_translation(static_translations, ru, ["feed_types", "categories"]) or ru
        localized[ru] = {
            "ru": ru,
            "en": translated_en,
        }

    return dict(sorted(localized.items(), key=lambda item: item[0].lower()))


def build_manifest(meta: dict[str, Any], feed_count: int, shard_payloads: dict[str, dict[str, Any]]) -> dict[str, Any]:
    return {
        "generated_from": "database/output source family",
        "consolidated_file": str(CONSOLIDATED_PATH.relative_to(ROOT)) if CONSOLIDATED_PATH.exists() else None,
        "category_files": {
            file_name: {
                "category_id": payload.get("category_id"),
                "count": payload.get("count", len(payload.get("feeds", []))),
            }
            for file_name, payload in shard_payloads.items()
        },
        "feed_count": feed_count,
        "meta_summary": {
            "nutrients": len((meta.get("nutrients") or {}).keys()),
            "units": len((meta.get("units") or {}).keys()),
            "categories": len((meta.get("categories") or {}).keys()),
            "regions": len((meta.get("regions") or {}).keys()),
        },
    }


def build_feed_taxonomy(meta: dict[str, Any], feed_records: list[dict[str, Any]]) -> dict[str, Any]:
    category_counts: dict[str, int] = {}
    for record in feed_records:
        category = record.get("category") or "other"
        category_counts[category] = category_counts.get(category, 0) + 1

    return {
        "raw_categories": meta.get("categories", {}),
        "runtime_categories": RUNTIME_CATEGORY_LABELS,
        "runtime_category_counts": category_counts,
    }


def dedupe_strings(values: list[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        normalized = normalized_text(value)
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        ordered.append(value.strip())
    return ordered


def build_price_key_record(feed_record: dict[str, Any]) -> dict[str, Any]:
    category = feed_record.get("category") or "other"
    category_labels = RUNTIME_CATEGORY_LABELS.get(category, RUNTIME_CATEGORY_LABELS["other"])
    aliases_ru = dedupe_strings(
        [
            str(feed_record.get("name_ru") or ""),
            str(feed_record.get("subcategory") or ""),
            category_labels["ru"],
        ]
    )
    aliases_en = dedupe_strings(
        [
            str(feed_record.get("name_en") or ""),
            str(feed_record.get("subcategory") or ""),
            category_labels["en"],
        ]
    )
    search_terms = dedupe_strings(aliases_ru + aliases_en)
    return {
        "feed_id": feed_record.get("id"),
        "source_id": feed_record.get("source_id"),
        "category": category,
        "subcategory": feed_record.get("subcategory"),
        "region": feed_record.get("region"),
        "aliases_ru": aliases_ru,
        "aliases_en": aliases_en,
        "search_terms": search_terms,
    }


def collect_observed_nutrient_keys(raw_feeds: list[dict[str, Any]]) -> set[str]:
    keys: set[str] = set()
    for raw_feed in raw_feeds:
        nutrition = raw_feed.get("nutrition") or {}
        if isinstance(nutrition, dict):
            keys.update(str(key) for key in nutrition.keys())
    return keys


def main() -> None:
    consolidated, raw_feeds, shard_payloads = load_source_family()
    meta = consolidated.get("meta", {})
    observed_nutrient_keys = collect_observed_nutrient_keys(raw_feeds)
    nutrient_registry = {
        key: value
        for key, value in (meta.get("nutrients", {}) or {}).items()
        if key in observed_nutrient_keys
    }
    static_translations = load_static_translations()

    feed_records = [build_feed_record(raw_feed, static_translations) for raw_feed in raw_feeds]
    catalog_records = [build_catalog_record(record) for record in feed_records]
    price_key_records = [build_price_key_record(record) for record in feed_records]
    subcategories = collect_subcategory_localization(raw_feeds, static_translations)

    shard_catalogs: dict[str, list[dict[str, Any]]] = {}
    for file_name, payload in shard_payloads.items():
        shard_catalogs[file_name] = [
            build_catalog_record(build_feed_record(raw_feed, static_translations))
            for raw_feed in payload.get("feeds", [])
        ]

    write_json(APP_DIR / "source_manifest.json", build_manifest(meta, len(feed_records), shard_payloads))
    write_json(APP_DIR / "nutrient_registry.json", nutrient_registry)
    write_json(APP_DIR / "feed_taxonomy.json", build_feed_taxonomy(meta, feed_records))
    write_json(APP_DIR / "feed_price_keys.json", price_key_records)
    write_json(APP_DIR / "feed_detail_profiles.json", feed_records)
    write_jsonl(APP_DIR / "feed_authority.jsonl", feed_records)
    write_json(RUNTIME_DIR / "catalog-lite.json", catalog_records)
    for file_name, records in shard_catalogs.items():
        write_json(RUNTIME_DIR / "category-shards" / file_name, records)

    write_json(FRONTEND_GENERATED_DIR / "feed-catalog.generated.json", catalog_records)
    write_json(FRONTEND_GENERATED_DIR / "feed-details.generated.json", feed_records)
    write_json(FRONTEND_GENERATED_DIR / "feed-price-keys.generated.json", price_key_records)
    write_json(
        FRONTEND_GENERATED_DIR / "feed-db-meta.generated.json",
        {
            "units": meta.get("units", {}),
            "nutrients": nutrient_registry,
            "categories": meta.get("categories", {}),
            "runtime_categories": RUNTIME_CATEGORY_LABELS,
            "regions": meta.get("regions", {}),
            "subcategories": subcategories,
        },
    )

    print(
        f"Generated feed runtime artifacts for {len(feed_records)} feeds from database/output source family."
    )


if __name__ == "__main__":
    main()
