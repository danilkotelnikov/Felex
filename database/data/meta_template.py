"""Generate the initial database metadata structure."""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from models import (  # noqa: E402
    AnimalTypeDef,
    CategoryDef,
    DatabaseMeta,
    NutrientDef,
    RegionDef,
    TranslatableText,
    UnitDef,
)


def create_meta_template() -> DatabaseMeta:
    """Create the initial database metadata structure."""

    units = {
        "fu": UnitDef(
            name=TranslatableText(ru="кормовые единицы", en="feed units"),
            type="energy",
        ),
        "MJ": UnitDef(
            name=TranslatableText(ru="МДж", en="megajoules"),
            type="energy",
            conversions={"kcal": 239.006},
        ),
        "g": UnitDef(
            name=TranslatableText(ru="г", en="grams"),
            type="mass",
            conversions={"oz": 0.035274, "kg": 0.001},
        ),
        "mg": UnitDef(
            name=TranslatableText(ru="мг", en="milligrams"),
            type="mass",
            conversions={"g": 0.001},
        ),
        "mcg": UnitDef(
            name=TranslatableText(ru="мкг", en="micrograms"),
            type="mass",
            conversions={"mg": 0.001},
        ),
        "IU": UnitDef(
            name=TranslatableText(ru="МЕ", en="international units"),
            type="biological",
        ),
        "thousand IU": UnitDef(
            name=TranslatableText(ru="тыс. МЕ", en="thousand international units"),
            type="biological",
            conversions={"IU": 1000},
        ),
        "million IU": UnitDef(
            name=TranslatableText(ru="млн. МЕ", en="million international units"),
            type="biological",
            conversions={"IU": 1000000},
        ),
        "percent": UnitDef(
            name=TranslatableText(ru="%", en="percent"),
            type="ratio",
        ),
    }

    nutrients = {
        "feed_units": NutrientDef(ru="Кормовые единицы", en="Feed units", group="energy", default_unit="fu"),
        "metabolizable_energy": NutrientDef(
            ru="Обменная энергия",
            en="Metabolizable energy",
            group="energy",
            default_unit="MJ",
            animal_specific=True,
        ),
        "dry_matter": NutrientDef(ru="Сухое вещество", en="Dry matter", group="composition", default_unit="g"),
        "crude_protein": NutrientDef(ru="Сырой протеин", en="Crude protein", group="composition", default_unit="g"),
        "digestible_protein": NutrientDef(
            ru="Переваримый протеин",
            en="Digestible protein",
            group="composition",
            default_unit="g",
            animal_specific=True,
        ),
        "lysine": NutrientDef(ru="Лизин", en="Lysine", group="amino_acids", default_unit="g"),
        "methionine_cystine": NutrientDef(
            ru="Метионин+цистин",
            en="Methionine+Cystine",
            group="amino_acids",
            default_unit="g",
        ),
        "crude_fiber": NutrientDef(ru="Сырая клетчатка", en="Crude fiber", group="composition", default_unit="g"),
        "sugars": NutrientDef(ru="Сахара", en="Sugars", group="composition", default_unit="g"),
        "starch": NutrientDef(ru="Крахмал", en="Starch", group="composition", default_unit="g"),
        "crude_fat": NutrientDef(ru="Сырой жир", en="Crude fat", group="composition", default_unit="g"),
        "calcium": NutrientDef(ru="Кальций", en="Calcium", group="minerals", default_unit="g"),
        "phosphorus": NutrientDef(ru="Фосфор", en="Phosphorus", group="minerals", default_unit="g"),
        "magnesium": NutrientDef(ru="Магний", en="Magnesium", group="minerals", default_unit="g"),
        "potassium": NutrientDef(ru="Калий", en="Potassium", group="minerals", default_unit="g"),
        "sodium": NutrientDef(ru="Натрий", en="Sodium", group="minerals", default_unit="g"),
        "sulfur": NutrientDef(ru="Сера", en="Sulfur", group="minerals", default_unit="g"),
        "iron": NutrientDef(ru="Железо", en="Iron", group="minerals", default_unit="mg"),
        "copper": NutrientDef(ru="Медь", en="Copper", group="minerals", default_unit="mg"),
        "zinc": NutrientDef(ru="Цинк", en="Zinc", group="minerals", default_unit="mg"),
        "manganese": NutrientDef(ru="Марганец", en="Manganese", group="minerals", default_unit="mg"),
        "cobalt": NutrientDef(ru="Кобальт", en="Cobalt", group="minerals", default_unit="mg"),
        "iodine": NutrientDef(ru="Йод", en="Iodine", group="minerals", default_unit="mg"),
        "carotene": NutrientDef(ru="Каротин", en="Carotene", group="vitamins", default_unit="mg"),
        "vitamin_d": NutrientDef(ru="Витамин D", en="Vitamin D", group="vitamins", default_unit="IU"),
        "vitamin_e": NutrientDef(ru="Витамин E", en="Vitamin E", group="vitamins", default_unit="mg"),
    }

    categories = {
        "green_feeds": CategoryDef(id="green_feeds", code="11", ru="Зеленые корма", en="Green feeds", has_regions=True),
        "rough_feeds": CategoryDef(id="rough_feeds", code="12", ru="Грубые корма", en="Rough feeds", has_regions=True),
        "succulent_feeds": CategoryDef(id="succulent_feeds", code="13", ru="Сочные корма", en="Succulent feeds", has_regions=True),
        "concentrated_feeds": CategoryDef(
            id="concentrated_feeds",
            code="14",
            ru="Концентрированные корма",
            en="Concentrated feeds",
            has_regions=True,
        ),
        "industrial_byproducts": CategoryDef(
            id="industrial_byproducts",
            code="15",
            ru="Отходы промышленности, пищевые",
            en="Industrial and food byproducts",
            has_regions=False,
        ),
        "animal_feeds": CategoryDef(
            id="animal_feeds",
            code="16",
            ru="Корма животного и микробного происхождения",
            en="Animal and microbial feeds",
            has_regions=False,
        ),
        "nitrogen_compounds": CategoryDef(
            id="nitrogen_compounds",
            code="17",
            ru="Небелковые азотистые соединения",
            en="Non-protein nitrogen compounds",
            has_regions=False,
        ),
        "mineral_supplements": CategoryDef(
            id="mineral_supplements",
            code="18",
            ru="Минеральные добавки",
            en="Mineral supplements",
            has_regions=False,
        ),
        "mixed_feeds": CategoryDef(
            id="mixed_feeds",
            code="19",
            ru="Комбикорма и заменители молока",
            en="Mixed feeds and milk replacers",
            has_regions=False,
        ),
    }

    regions = {
        "russia_avg": RegionDef(id="russia_avg", ru="В среднем по России", en="Russia average"),
        "northwest_north": RegionDef(
            id="northwest_north",
            ru="Северо-Западный, Северный экономические районы",
            en="Northwestern and Northern economic regions",
        ),
        "central": RegionDef(id="central", ru="Центральный экономический район", en="Central economic region"),
        "volga_vyatka": RegionDef(
            id="volga_vyatka",
            ru="Волго-Вятский экономический район",
            en="Volga-Vyatka economic region",
        ),
        "central_black_earth": RegionDef(
            id="central_black_earth",
            ru="Центрально-черноземный экономический район",
            en="Central Black Earth region",
        ),
        "volga": RegionDef(id="volga", ru="Поволжский экономический район", en="Volga region"),
        "north_caucasus": RegionDef(
            id="north_caucasus",
            ru="Северо-Кавказский экономический район",
            en="North Caucasus region",
        ),
        "urals": RegionDef(id="urals", ru="Уральский экономический район", en="Urals region"),
        "west_siberia": RegionDef(
            id="west_siberia",
            ru="Западно-Сибирский экономический район",
            en="West Siberian region",
        ),
        "east_siberia": RegionDef(
            id="east_siberia",
            ru="Восточно-Сибирский экономический район",
            en="East Siberian region",
        ),
        "far_east": RegionDef(id="far_east", ru="Дальневосточный экономический район", en="Far Eastern region"),
    }

    animal_types = {
        "cattle": AnimalTypeDef(ru="КРС", en="Cattle"),
        "swine": AnimalTypeDef(ru="Свиньи", en="Swine"),
        "sheep": AnimalTypeDef(ru="Овцы", en="Sheep"),
        "goats": AnimalTypeDef(ru="Козы", en="Goats"),
        "poultry": AnimalTypeDef(ru="Птица", en="Poultry"),
        "chickens": AnimalTypeDef(ru="Куры", en="Chickens"),
        "ducks": AnimalTypeDef(ru="Утки", en="Ducks"),
        "geese": AnimalTypeDef(ru="Гуси", en="Geese"),
        "turkeys": AnimalTypeDef(ru="Индейки", en="Turkeys"),
        "horses": AnimalTypeDef(ru="Лошади", en="Horses"),
        "rabbits": AnimalTypeDef(ru="Кролики", en="Rabbits"),
        "fish": AnimalTypeDef(ru="Рыба", en="Fish"),
        "fur_animals": AnimalTypeDef(ru="Пушные звери", en="Fur animals"),
    }

    return DatabaseMeta(
        units=units,
        nutrients=nutrients,
        animal_types=animal_types,
        categories=categories,
        regions=regions,
        generated=datetime.now(timezone.utc).isoformat(),
    )


if __name__ == "__main__":
    meta = create_meta_template()
    print(json.dumps(meta.model_dump(), indent=2, ensure_ascii=False))
