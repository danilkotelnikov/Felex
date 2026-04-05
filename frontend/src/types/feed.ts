export type FeedSourceKind = 'normalized' | 'curated' | 'custom' | 'imported';
export type FeedTranslationStatus = 'ready' | 'source_only';
export type FeedProfileStatus = 'complete' | 'partial' | 'limited';
export type FeedProfileSectionKey = 'energy' | 'protein' | 'fiber' | 'minerals' | 'vitamins';
export type FeedProfileSectionStatus = 'present' | 'partial' | 'missing';
export type FeedSuitabilityStatus = 'appropriate' | 'conditional' | 'restricted';
export type FeedCriticalNutrientKey =
  | 'dry_matter'
  | 'energy_oe_cattle'
  | 'energy_oe_pig'
  | 'energy_oe_poultry'
  | 'crude_protein'
  | 'dig_protein_cattle'
  | 'calcium'
  | 'phosphorus'
  | 'vit_d3'
  | 'vit_e'
  | 'lysine'
  | 'methionine_cystine';

export interface FeedSourceUnitValue {
  value: number;
  unit: string;
}

export type FeedSourceNutrient =
  | FeedSourceUnitValue
  | Record<string, FeedSourceUnitValue>;

export interface FeedProfileSectionAudit {
  key: FeedProfileSectionKey;
  status: FeedProfileSectionStatus;
  present: number;
  expected: number;
}

export interface FeedCriticalCoverageAudit {
  species: string;
  stage_context?: string | null;
  coverage_status: FeedProfileStatus;
  present_required: number;
  required_total: number;
  required_keys: FeedCriticalNutrientKey[];
  missing_keys: FeedCriticalNutrientKey[];
}

export interface Feed {
  id: number;
  source_id?: string;
  source_url?: string;
  name_ru: string;
  name_en?: string;
  category?: string;
  subcategory?: string;
  source_category_id?: string;
  source_subcategory_en?: string;
  source_nutrition?: Record<string, FeedSourceNutrient>;

  dry_matter?: number;
  energy_oe_cattle?: number;
  energy_oe_pig?: number;
  energy_oe_poultry?: number;
  koe?: number;

  crude_protein?: number;
  dig_protein_cattle?: number;
  dig_protein_pig?: number;
  dig_protein_poultry?: number;
  lysine?: number;
  methionine_cystine?: number;

  crude_fat?: number;
  crude_fiber?: number;
  starch?: number;
  sugar?: number;

  calcium?: number;
  phosphorus?: number;
  magnesium?: number;
  potassium?: number;
  sodium?: number;
  sulfur?: number;
  iron?: number;
  copper?: number;
  zinc?: number;
  manganese?: number;
  cobalt?: number;
  iodine?: number;
  carotene?: number;

  vit_d3?: number;
  vit_e?: number;

  moisture?: number;
  feed_conversion?: number;
  palatability?: number;
  max_inclusion_cattle?: number;
  max_inclusion_pig?: number;
  max_inclusion_poultry?: number;

  price_per_ton?: number;
  price_updated_at?: string;
  region?: string;

  is_custom?: boolean;
  verified?: boolean;
  notes?: string;
  source_kind?: FeedSourceKind;
  translation_status?: FeedTranslationStatus;
  profile_status?: FeedProfileStatus;
  profile_sections?: FeedProfileSectionAudit[];
  critical_nutrient_audit?: FeedCriticalCoverageAudit;
  suitability_status?: FeedSuitabilityStatus;
  suitability_notes?: string[];
  suitability_max_inclusion_pct?: number;
}

export interface FeedCategory {
  id: string;
  name_ru: string;
  name_en: string;
  count: number;
}

export interface FeedListResponse {
  data: Feed[];
  total: number;
  limit: number;
  offset: number;
}

export const FEED_CATEGORIES: Record<string, { name_ru: string; name_en: string }> = {
  grain: { name_ru: 'Зерновые', name_en: 'Grains' },
  concentrate: { name_ru: 'Концентраты', name_en: 'Concentrates' },
  oilseed_meal: { name_ru: 'Жмыхи и шроты', name_en: 'Oilseed meals' },
  protein: { name_ru: 'Белковые', name_en: 'Protein feeds' },
  roughage: { name_ru: 'Грубые корма', name_en: 'Roughage' },
  silage: { name_ru: 'Силос и сенаж', name_en: 'Silage' },
  succulent: { name_ru: 'Сочные корма', name_en: 'Succulent feeds' },
  green_forage: { name_ru: 'Зелёные корма', name_en: 'Green forages' },
  animal_origin: { name_ru: 'Животного происхождения', name_en: 'Animal origin' },
  mineral: { name_ru: 'Минеральные', name_en: 'Minerals' },
  premix: { name_ru: 'Премиксы', name_en: 'Premixes' },
  oil_fat: { name_ru: 'Масла и жиры', name_en: 'Oils & fats' },
  byproduct: { name_ru: 'Побочные продукты', name_en: 'By-products' },
  compound_feed: { name_ru: 'Комбикорма', name_en: 'Compound feeds' },
  additive: { name_ru: 'Кормовые добавки', name_en: 'Feed additives' },
  other: { name_ru: 'Прочие', name_en: 'Other' },
};
