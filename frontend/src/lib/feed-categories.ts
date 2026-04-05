/**
 * Feed category display utilities with multi-locale support.
 */

export type FeedCategoryId =
  | 'grain'
  | 'concentrate'
  | 'oilseed_meal'
  | 'protein'
  | 'roughage'
  | 'silage'
  | 'succulent'
  | 'green_forage'
  | 'animal_origin'
  | 'mineral'
  | 'premix'
  | 'oil_fat'
  | 'byproduct'
  | 'compound_feed'
  | 'additive'
  | 'root_crops'
  | 'other';

export type CategoryLocale = 'en' | 'ru';

export interface CategoryLabelMap {
  en: string;
  ru: string;
}

/** Localized display labels for every feed category slug. */
export const CATEGORY_LABELS: Record<FeedCategoryId, CategoryLabelMap> = {
  grain: { en: 'Grains', ru: 'Зерновые' },
  concentrate: { en: 'Concentrates', ru: 'Концентраты' },
  oilseed_meal: { en: 'Oilseed meals', ru: 'Жмыхи и шроты' },
  protein: { en: 'Protein feeds', ru: 'Белковые корма' },
  roughage: { en: 'Roughage', ru: 'Грубые корма' },
  silage: { en: 'Silage', ru: 'Силос' },
  succulent: { en: 'Succulent feeds', ru: 'Сочные корма' },
  green_forage: { en: 'Green forage', ru: 'Зелёные корма' },
  animal_origin: { en: 'Animal-origin feeds', ru: 'Корма животного происхождения' },
  mineral: { en: 'Mineral supplements', ru: 'Минеральные добавки' },
  premix: { en: 'Premixes', ru: 'Премиксы' },
  oil_fat: { en: 'Oils & fats', ru: 'Масла и жиры' },
  byproduct: { en: 'Byproducts', ru: 'Побочные продукты' },
  compound_feed: { en: 'Compound feeds', ru: 'Комбикорма' },
  additive: { en: 'Feed additives', ru: 'Кормовые добавки' },
  root_crops: { en: 'Root crops', ru: 'Корнеплоды' },
  other: { en: 'Other', ru: 'Прочие' },
};

/**
 * Returns the localized display label for a feed category.
 *
 * @param category - internal category slug (e.g. `"grain"`)
 * @param locale   - display locale, `"en"` or `"ru"` (defaults to `"en"`)
 * @returns localized label, or the raw slug if the category is unknown
 */
export function getCategoryLabel(
  category: string,
  locale: CategoryLocale = 'en',
): string {
  const entry = CATEGORY_LABELS[category as FeedCategoryId];
  if (!entry) {
    return category;
  }
  return entry[locale] ?? entry.en;
}

/** Alias for getCategoryLabel, used by multiple components. */
export const getFeedCategoryLabel = getCategoryLabel;

/** Language type alias used by feed detail registry. */
export type FeedCategoryLanguage = CategoryLocale;

/**
 * Returns a localized display label for a feed subcategory.
 * Falls back to the raw subcategory string if no mapping exists.
 */
export function getFeedSubcategoryLabel(
  subcategory: string | undefined | null,
  _locale: CategoryLocale = 'en',
): string {
  if (!subcategory || !subcategory.trim()) {
    return '';
  }
  return subcategory;
}

/** All known feed category slugs in their canonical order. */
export const FEED_CATEGORY_IDS: readonly FeedCategoryId[] = [
  'grain',
  'concentrate',
  'oilseed_meal',
  'protein',
  'roughage',
  'silage',
  'succulent',
  'green_forage',
  'animal_origin',
  'mineral',
  'premix',
  'oil_fat',
  'byproduct',
  'compound_feed',
  'additive',
  'root_crops',
  'other',
] as const;
