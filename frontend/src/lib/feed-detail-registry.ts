import type { Feed, FeedSourceNutrient, FeedSourceUnitValue } from '@/types/feed';
import {
  FEED_DB_NUTRIENTS,
  FEED_DB_UNITS,
  getDbLocalizedText,
  type FeedDbLocalizedText,
} from '@/lib/feed-db-metadata';

import type { FeedCategoryLanguage } from './feed-categories';

export interface FeedDetailRow {
  id: string;
  label: string;
  value: string;
  indent?: boolean;
}

export interface FeedDetailSection {
  title: string;
  rows: FeedDetailRow[];
}

interface SourceSectionDef {
  title: { ru: string; en: string };
  keys: string[];
}

const SOURCE_SECTION_DEFS: SourceSectionDef[] = [
  {
    title: { ru: 'Энергия и сухое вещество', en: 'Energy and dry matter' },
    keys: ['feed_units', 'metabolizable_energy', 'dry_matter'],
  },
  {
    title: { ru: 'Протеин и аминокислоты', en: 'Protein and amino acids' },
    keys: [
      'crude_protein',
      'digestible_protein',
      'lysine',
      'methionine_cystine',
    ],
  },
  {
    title: { ru: 'Клетчатка, жир и углеводы', en: 'Fiber, fat, and carbohydrates' },
    keys: [
      'crude_fiber',
      'starch',
      'sugars',
      'crude_fat',
    ],
  },
  {
    title: { ru: 'Минералы и микроэлементы', en: 'Minerals and trace elements' },
    keys: [
      'calcium',
      'phosphorus',
      'magnesium',
      'potassium',
      'sodium',
      'sulfur',
      'iron',
      'copper',
      'zinc',
      'manganese',
      'cobalt',
      'iodine',
    ],
  },
  {
    title: { ru: 'Витамины и кофакторы', en: 'Vitamins and cofactors' },
    keys: [
      'carotene',
      'vitamin_d',
      'vitamin_e',
    ],
  },
];

const ANIMAL_LABELS: Record<string, FeedDbLocalizedText> = {
  cattle: { ru: 'КРС', en: 'Cattle' },
  swine: { ru: 'Свиньи', en: 'Swine' },
  sheep: { ru: 'Овцы', en: 'Sheep' },
  goats: { ru: 'Козы', en: 'Goats' },
  poultry: { ru: 'Птица', en: 'Poultry' },
  chickens: { ru: 'Куры', en: 'Chickens' },
  ducks: { ru: 'Утки', en: 'Ducks' },
  geese: { ru: 'Гуси', en: 'Geese' },
  turkeys: { ru: 'Индейки', en: 'Turkeys' },
  horses: { ru: 'Лошади', en: 'Horses' },
  rabbits: { ru: 'Кролики', en: 'Rabbits' },
  fish: { ru: 'Рыба', en: 'Fish' },
  universal: { ru: 'Общее', en: 'Universal' },
};

// Compound unit localization for legacy feed detail display
const COMPOUND_UNITS: Record<string, FeedDbLocalizedText> = {
  '%': { ru: '%', en: '%' },
  'MJ/kg DM': { ru: 'МДж/кг СВ', en: 'MJ/kg DM' },
  'g/kg': { ru: 'г/кг', en: 'g/kg' },
  'mg/kg': { ru: 'мг/кг', en: 'mg/kg' },
  'IU/kg': { ru: 'МЕ/кг', en: 'IU/kg' },
  'mcg/kg': { ru: 'мкг/кг', en: 'mcg/kg' },
};

type FeedNumericKey = {
  [K in keyof Feed]: Feed[K] extends number | undefined ? K : never;
}[keyof Feed];

interface LegacySectionDef {
  title: { ru: string; en: string };
  rows: Array<{ key: FeedNumericKey; label: { ru: string; en: string }; unit: string }>;
}

const LEGACY_SECTIONS: LegacySectionDef[] = [
  {
    title: { ru: 'Энергия', en: 'Energy' },
    rows: [
      { key: 'dry_matter', label: { ru: 'Сухое вещество', en: 'Dry matter' }, unit: '%' },
      { key: 'energy_oe_cattle', label: { ru: 'ОЭ КРС', en: 'ME cattle' }, unit: 'MJ/kg DM' },
      { key: 'energy_oe_pig', label: { ru: 'ОЭ свиней', en: 'ME swine' }, unit: 'MJ/kg DM' },
      { key: 'energy_oe_poultry', label: { ru: 'ОЭ птицы', en: 'ME poultry' }, unit: 'MJ/kg DM' },
      { key: 'koe', label: { ru: 'Кормовые единицы', en: 'Feed units' }, unit: '' },
    ],
  },
  {
    title: { ru: 'Протеин и аминокислоты', en: 'Protein and amino acids' },
    rows: [
      { key: 'crude_protein', label: { ru: 'Сырой протеин', en: 'Crude protein' }, unit: 'g/kg' },
      { key: 'dig_protein_cattle', label: { ru: 'Переваримый протеин, КРС', en: 'Digestible protein, cattle' }, unit: 'g/kg' },
      { key: 'dig_protein_pig', label: { ru: 'Переваримый протеин, свиньи', en: 'Digestible protein, swine' }, unit: 'g/kg' },
      { key: 'dig_protein_poultry', label: { ru: 'Переваримый протеин, птица', en: 'Digestible protein, poultry' }, unit: 'g/kg' },
      { key: 'lysine', label: { ru: 'Лизин', en: 'Lysine' }, unit: 'g/kg' },
      { key: 'methionine_cystine', label: { ru: 'Метионин + цистин', en: 'Methionine + cystine' }, unit: 'g/kg' },
    ],
  },
  {
    title: { ru: 'Минералы и витамины', en: 'Minerals and vitamins' },
    rows: [
      { key: 'calcium', label: { ru: 'Кальций', en: 'Calcium' }, unit: 'g/kg' },
      { key: 'phosphorus', label: { ru: 'Фосфор', en: 'Phosphorus' }, unit: 'g/kg' },
      { key: 'magnesium', label: { ru: 'Магний', en: 'Magnesium' }, unit: 'g/kg' },
      { key: 'potassium', label: { ru: 'Калий', en: 'Potassium' }, unit: 'g/kg' },
      { key: 'sodium', label: { ru: 'Натрий', en: 'Sodium' }, unit: 'g/kg' },
      { key: 'sulfur', label: { ru: 'Сера', en: 'Sulfur' }, unit: 'g/kg' },
      { key: 'iron', label: { ru: 'Железо', en: 'Iron' }, unit: 'mg/kg' },
      { key: 'zinc', label: { ru: 'Цинк', en: 'Zinc' }, unit: 'mg/kg' },
      { key: 'manganese', label: { ru: 'Марганец', en: 'Manganese' }, unit: 'mg/kg' },
      { key: 'copper', label: { ru: 'Медь', en: 'Copper' }, unit: 'mg/kg' },
      { key: 'cobalt', label: { ru: 'Кобальт', en: 'Cobalt' }, unit: 'mg/kg' },
      { key: 'iodine', label: { ru: 'Йод', en: 'Iodine' }, unit: 'mg/kg' },
      { key: 'carotene', label: { ru: 'Каротин', en: 'Carotene' }, unit: 'mg/kg' },
      { key: 'vit_d3', label: { ru: 'Витамин D3', en: 'Vitamin D3' }, unit: 'IU/kg' },
      { key: 'vit_e', label: { ru: 'Витамин E', en: 'Vitamin E' }, unit: 'mg/kg' },
    ],
  },
];

function localizeText(text: { ru: string; en: string }, language: FeedCategoryLanguage): string {
  return language === 'ru' ? text.ru : text.en;
}

function humanizeSourceKey(key: string, language: FeedCategoryLanguage): string {
  const text = key.replace(/_/g, ' ').trim();
  if (!text) {
    return key;
  }
  if (language === 'ru') {
    return text;
  }
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function nutrientLabel(key: string, language: FeedCategoryLanguage): string {
  const meta = FEED_DB_NUTRIENTS[key];
  const localized = getDbLocalizedText(meta, language);
  return localized ?? humanizeSourceKey(key, language);
}

function unitLabel(unit: string, language: FeedCategoryLanguage): string {
  if (!unit.trim()) {
    return '';
  }
  const meta = FEED_DB_UNITS[unit];
  const localized = meta ? getDbLocalizedText(meta.name, language) : null;
  return localized ?? unit;
}

function compoundUnitLabel(unit: string, language: FeedCategoryLanguage): string {
  if (!unit.trim()) {
    return '';
  }
  const localized = getDbLocalizedText(COMPOUND_UNITS[unit], language);
  return localized ?? unit;
}

function animalLabel(key: string, language: FeedCategoryLanguage): string {
  const localized = getDbLocalizedText(ANIMAL_LABELS[key], language);
  return localized ?? key;
}

function isUnitValue(value: FeedSourceNutrient): value is FeedSourceUnitValue {
  return typeof value === 'object' && value !== null && 'value' in value;
}

function formatNumericValue(value: number, language: FeedCategoryLanguage): string {
  const locale = language === 'ru' ? 'ru-RU' : 'en-US';
  return value.toLocaleString(locale, {
    minimumFractionDigits: value >= 1000 ? 0 : 0,
    maximumFractionDigits: value >= 100 ? 1 : 2,
  });
}

function formatSourceValue(
  value: FeedSourceUnitValue,
  language: FeedCategoryLanguage,
): string {
  const formattedUnit = unitLabel(value.unit, language);
  const formattedValue = formatNumericValue(value.value, language);
  return formattedUnit ? `${formattedValue} ${formattedUnit}` : formattedValue;
}

function buildSourceRows(
  nutrientKey: string,
  nutrient: FeedSourceNutrient,
  language: FeedCategoryLanguage,
): FeedDetailRow[] {
  const baseLabel = nutrientLabel(nutrientKey, language);
  if (isUnitValue(nutrient)) {
    return [{
      id: nutrientKey,
      label: baseLabel,
      value: formatSourceValue(nutrient, language),
    }];
  }

  return Object.entries(nutrient)
    .filter(([, value]) => isUnitValue(value))
    .map(([animalKey, value]) => ({
      id: `${nutrientKey}:${animalKey}`,
      label: `${baseLabel} (${animalLabel(animalKey, language)})`,
      value: formatSourceValue(value, language),
      indent: true,
    }));
}

function buildLegacySections(
  feed: Feed,
  language: FeedCategoryLanguage,
): FeedDetailSection[] {
  return LEGACY_SECTIONS.map((section) => ({
    title: localizeText(section.title, language),
    rows: section.rows
      .map((row) => {
        const value = feed[row.key as keyof Feed];
        if (typeof value !== 'number' || !Number.isFinite(value)) {
          return null;
        }
        const localizedUnit = compoundUnitLabel(row.unit, language);
        return {
          id: String(row.key),
          label: localizeText(row.label, language),
          value: `${formatNumericValue(value, language)}${localizedUnit ? ` ${localizedUnit}` : ''}`,
        } satisfies FeedDetailRow;
      })
      .filter((row): row is FeedDetailRow => Boolean(row)),
  })).filter((section) => section.rows.length > 0);
}

export function getFeedDetailSections(
  feed: Feed,
  language: FeedCategoryLanguage,
): FeedDetailSection[] {
  const sourceNutrition = feed.source_nutrition;
  if (!sourceNutrition || Object.keys(sourceNutrition).length === 0) {
    return buildLegacySections(feed, language);
  }

  const usedKeys = new Set<string>();
  const sections = SOURCE_SECTION_DEFS.map((section) => {
    const rows = section.keys.flatMap((key) => {
      const nutrient = sourceNutrition[key];
      if (!nutrient) {
        return [];
      }
      usedKeys.add(key);
      return buildSourceRows(key, nutrient, language);
    });

    return {
      title: localizeText(section.title, language),
      rows,
    };
  }).filter((section) => section.rows.length > 0);

  return sections;
}

export function getFeedPriceUnit(language: FeedCategoryLanguage): string {
  return language === 'ru' ? '₽/т' : 'RUB/t';
}
