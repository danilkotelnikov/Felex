import FEED_DB_META from '@/generated/feed-db-meta.generated.json';

export interface FeedDbLocalizedText {
  ru?: string;
  en?: string;
}

export interface FeedDbCategoryMeta {
  id?: string;
  code?: string;
  ru?: string;
  en?: string;
  has_regions?: boolean;
}

export interface FeedDbRuntimeCategoryMeta {
  ru: string;
  en: string;
}

export interface FeedDbUnitMeta {
  name?: FeedDbLocalizedText;
  type?: string;
  conversions?: Record<string, number>;
}

export interface FeedDbNutrientMeta {
  ru?: string;
  en?: string;
  group?: string;
  default_unit?: string;
  animal_specific?: boolean;
}

interface FeedDbMetaPayload {
  units?: Record<string, FeedDbUnitMeta>;
  nutrients?: Record<string, FeedDbNutrientMeta>;
  categories?: Record<string, FeedDbCategoryMeta>;
  runtime_categories?: Record<string, FeedDbRuntimeCategoryMeta>;
  regions?: Record<string, FeedDbLocalizedText>;
  subcategories?: Record<string, FeedDbLocalizedText>;
}

const FEED_DB = FEED_DB_META as FeedDbMetaPayload;

export const FEED_DB_UNITS = FEED_DB.units ?? {};
export const FEED_DB_NUTRIENTS = FEED_DB.nutrients ?? {};
export const FEED_DB_CATEGORIES = FEED_DB.categories ?? {};
export const FEED_DB_RUNTIME_CATEGORIES = FEED_DB.runtime_categories ?? {};
export const FEED_DB_REGIONS = FEED_DB.regions ?? {};
export const FEED_DB_SUBCATEGORIES = FEED_DB.subcategories ?? {};

export function getDbLocalizedText(
  labels: FeedDbLocalizedText | null | undefined,
  language: 'ru' | 'en',
): string | null {
  if (!labels) {
    return null;
  }
  const preferred = language === 'en' ? labels.en?.trim() : labels.ru?.trim();
  if (preferred) {
    return preferred;
  }
  const fallback = language === 'en' ? labels.ru?.trim() : labels.en?.trim();
  return fallback || null;
}

