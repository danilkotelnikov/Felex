"""Pydantic models for the feed database."""
from __future__ import annotations

from typing import Any, Optional

from pydantic import BaseModel, Field


class TranslatableText(BaseModel):
    """Bilingual text value."""

    ru: str
    en: str = ""


class NutrientValue(BaseModel):
    """Single nutrient value and its unit."""

    value: float
    unit: str


class AnimalSpecificValue(BaseModel):
    """Nutrient value that can vary by animal type."""

    cattle: Optional[NutrientValue] = None
    swine: Optional[NutrientValue] = None
    sheep: Optional[NutrientValue] = None
    goats: Optional[NutrientValue] = None
    poultry: Optional[NutrientValue] = None
    chickens: Optional[NutrientValue] = None
    ducks: Optional[NutrientValue] = None
    geese: Optional[NutrientValue] = None
    turkeys: Optional[NutrientValue] = None
    horses: Optional[NutrientValue] = None
    rabbits: Optional[NutrientValue] = None
    fish: Optional[NutrientValue] = None
    fur_animals: Optional[NutrientValue] = None
    mink: Optional[NutrientValue] = None
    fox: Optional[NutrientValue] = None
    universal: Optional[NutrientValue] = None


class ParseError(BaseModel):
    """Parse error details."""

    field: str
    error: str
    raw_value: Optional[str] = None
    expected_unit: Optional[str] = None
    found_unit: Optional[str] = None
    message: str


class FeedItem(BaseModel):
    """Individual feed item with nutritional data."""

    id: str
    name: TranslatableText
    category_id: str
    subcategory: TranslatableText
    region_id: Optional[str] = None
    nutrition: dict[str, NutrientValue | AnimalSpecificValue | dict[str, Any]] = Field(
        default_factory=dict
    )
    source_url: str
    parse_errors: list[ParseError] = Field(default_factory=list)


class UnitDef(BaseModel):
    """Unit definition."""

    name: TranslatableText
    type: str
    conversions: Optional[dict[str, float]] = None


class NutrientDef(BaseModel):
    """Nutrient definition."""

    ru: str
    en: str
    group: str
    default_unit: str
    animal_specific: bool = False


class CategoryDef(BaseModel):
    """Category definition."""

    id: str
    code: str
    ru: str
    en: str
    has_regions: bool


class RegionDef(BaseModel):
    """Region definition."""

    id: str
    ru: str
    en: str


class AnimalTypeDef(BaseModel):
    """Animal type definition."""

    ru: str
    en: str


class DatabaseMeta(BaseModel):
    """Database metadata."""

    units: dict[str, UnitDef]
    nutrients: dict[str, NutrientDef]
    animal_types: dict[str, AnimalTypeDef]
    categories: dict[str, CategoryDef]
    regions: dict[str, RegionDef]
    generated: str
    source: str = "vidkormov.narod.ru"
    version: str = "1.0"
    total_feeds: int = 0


class FeedDatabase(BaseModel):
    """Complete database payload."""

    meta: DatabaseMeta
    feeds: list[FeedItem] = Field(default_factory=list)


class UrlEntry(BaseModel):
    """Tracked URL discovered during crawl."""

    url: str
    category_id: str
    subcategory_ru: str
    region_id: Optional[str] = None
    region_ru: Optional[str] = None
    feed_name_ru: Optional[str] = None
    status: str = "pending"
    error_message: Optional[str] = None


class ProgressState(BaseModel):
    """Persisted parser state."""

    phase: str = "discovery"
    discovered_urls: list[UrlEntry] = Field(default_factory=list)
    total_discovered: int = 0
    total_parsed: int = 0
    total_failed: int = 0
