import type { NormRange } from '@/types/nutrient';

export type NutrientLanguage = 'ru' | 'en';
export type NutrientFamily = 'cattle' | 'swine' | 'poultry';
export type NutrientDisplayBasis = 'absolute' | 'per_kg_feed' | 'percent_of_feed' | 'percent_of_dm';
type NutrientUnitKey =
  | 'none'
  | 'eke'
  | 'mj'
  | 'g'
  | 'mg'
  | 'iu'
  | 'kg'
  | 'kg_dm'
  | 'pct'
  | 'pct_dm'
  | 'ratio';
type NutrientCategory = 'energy' | 'protein' | 'fiber' | 'mineral' | 'vitamin' | 'intake' | 'ratio';

interface NutrientMeta {
  key: string;
  name_ru: string;
  name_en: string;
  unit_key: NutrientUnitKey;
  category: NutrientCategory;
}

const UNIT_LABELS: Record<NutrientUnitKey, { ru: string; en: string }> = {
  none: { ru: '', en: '' },
  eke: { ru: 'ЭКЕ', en: 'EKE' },
  mj: { ru: 'МДж', en: 'MJ' },
  g: { ru: 'г', en: 'g' },
  mg: { ru: 'мг', en: 'mg' },
  iu: { ru: 'МЕ', en: 'IU' },
  kg: { ru: 'кг', en: 'kg' },
  kg_dm: { ru: 'кг СВ', en: 'kg DM' },
  pct: { ru: '%', en: '%' },
  pct_dm: { ru: '% СВ', en: '% DM' },
  ratio: { ru: '', en: '' },
};

const BASE_NUTRIENTS: NutrientMeta[] = [
  { key: 'dry_matter', name_ru: 'Сухое вещество', name_en: 'Dry matter', unit_key: 'pct', category: 'intake' },
  { key: 'koe', name_ru: 'Кормовые единицы', name_en: 'Feed units', unit_key: 'eke', category: 'energy' },
  { key: 'energy_eke', name_ru: 'ЭКЕ', name_en: 'Energy units', unit_key: 'eke', category: 'energy' },
  { key: 'energy_oe_cattle', name_ru: 'ОЭ КРС', name_en: 'ME cattle', unit_key: 'mj', category: 'energy' },
  { key: 'energy_oe_pig', name_ru: 'ОЭ свиней', name_en: 'ME swine', unit_key: 'mj', category: 'energy' },
  { key: 'energy_oe_poultry', name_ru: 'ОЭ птицы', name_en: 'ME poultry', unit_key: 'mj', category: 'energy' },
  { key: 'crude_protein', name_ru: 'Сырой протеин', name_en: 'Crude protein', unit_key: 'g', category: 'protein' },
  { key: 'dig_protein_cattle', name_ru: 'Переваримый протеин, КРС', name_en: 'Digestible protein, cattle', unit_key: 'g', category: 'protein' },
  { key: 'dig_protein_pig', name_ru: 'Переваримый протеин, свиньи', name_en: 'Digestible protein, swine', unit_key: 'g', category: 'protein' },
  { key: 'dig_protein_poultry', name_ru: 'Переваримый протеин, птица', name_en: 'Digestible protein, poultry', unit_key: 'g', category: 'protein' },
  { key: 'lysine', name_ru: 'Лизин', name_en: 'Lysine', unit_key: 'g', category: 'protein' },
  { key: 'methionine_cystine', name_ru: 'Метионин + цистин', name_en: 'Methionine + cystine', unit_key: 'g', category: 'protein' },
  { key: 'crude_fat', name_ru: 'Сырой жир', name_en: 'Crude fat', unit_key: 'g', category: 'fiber' },
  { key: 'crude_fiber', name_ru: 'Сырая клетчатка', name_en: 'Crude fiber', unit_key: 'g', category: 'fiber' },
  { key: 'starch', name_ru: 'Крахмал', name_en: 'Starch', unit_key: 'g', category: 'fiber' },
  { key: 'sugar', name_ru: 'Сахар', name_en: 'Sugar', unit_key: 'g', category: 'fiber' },
  { key: 'calcium', name_ru: 'Кальций', name_en: 'Calcium', unit_key: 'g', category: 'mineral' },
  { key: 'phosphorus', name_ru: 'Фосфор', name_en: 'Phosphorus', unit_key: 'g', category: 'mineral' },
  { key: 'magnesium', name_ru: 'Магний', name_en: 'Magnesium', unit_key: 'g', category: 'mineral' },
  { key: 'potassium', name_ru: 'Калий', name_en: 'Potassium', unit_key: 'g', category: 'mineral' },
  { key: 'sodium', name_ru: 'Натрий', name_en: 'Sodium', unit_key: 'g', category: 'mineral' },
  { key: 'sulfur', name_ru: 'Сера', name_en: 'Sulfur', unit_key: 'g', category: 'mineral' },
  { key: 'iron', name_ru: 'Железо', name_en: 'Iron', unit_key: 'mg', category: 'mineral' },
  { key: 'copper', name_ru: 'Медь', name_en: 'Copper', unit_key: 'mg', category: 'mineral' },
  { key: 'zinc', name_ru: 'Цинк', name_en: 'Zinc', unit_key: 'mg', category: 'mineral' },
  { key: 'manganese', name_ru: 'Марганец', name_en: 'Manganese', unit_key: 'mg', category: 'mineral' },
  { key: 'cobalt', name_ru: 'Кобальт', name_en: 'Cobalt', unit_key: 'mg', category: 'mineral' },
  { key: 'iodine', name_ru: 'Йод', name_en: 'Iodine', unit_key: 'mg', category: 'mineral' },
  { key: 'carotene', name_ru: 'Каротин', name_en: 'Carotene', unit_key: 'mg', category: 'vitamin' },
  { key: 'vit_d3', name_ru: 'Витамин D3', name_en: 'Vitamin D3', unit_key: 'iu', category: 'vitamin' },
  { key: 'vit_e', name_ru: 'Витамин E', name_en: 'Vitamin E', unit_key: 'mg', category: 'vitamin' },
  { key: 'selenium', name_ru: 'Селен', name_en: 'Selenium', unit_key: 'mg', category: 'mineral' },
];


const DERIVED_NUTRIENTS: NutrientMeta[] = [
  { key: 'dry_matter_intake', name_ru: 'Потребление сухого вещества', name_en: 'Dry matter intake', unit_key: 'kg_dm', category: 'intake' },
  { key: 'feed_intake', name_ru: 'Потребление корма', name_en: 'Feed intake', unit_key: 'kg', category: 'intake' },
  { key: 'dm_pct', name_ru: 'Доля сухого вещества', name_en: 'Dry matter share', unit_key: 'pct', category: 'intake' },
  { key: 'crude_protein_pct', name_ru: 'Сырой протеин', name_en: 'Crude protein', unit_key: 'pct', category: 'protein' },
  { key: 'cp_pct_dm', name_ru: 'Сырой протеин', name_en: 'Crude protein', unit_key: 'pct_dm', category: 'protein' },
  { key: 'dig_protein_cattle_pct_cp', name_ru: 'Доля переваримого протеина', name_en: 'Digestible protein share', unit_key: 'pct', category: 'protein' },
  { key: 'methionine_cystine_lys_ratio', name_ru: 'Мет + цис : лизин', name_en: 'Met + Cys : lysine', unit_key: 'ratio', category: 'ratio' },
  { key: 'starch_pct_dm', name_ru: 'Крахмал', name_en: 'Starch', unit_key: 'pct_dm', category: 'fiber' },
  { key: 'calcium_pct', name_ru: 'Кальций', name_en: 'Calcium', unit_key: 'pct', category: 'mineral' },
  { key: 'ca_p_ratio', name_ru: 'Ca:P', name_en: 'Ca:P', unit_key: 'ratio', category: 'ratio' },
];

const BASE_BY_KEY = new Map(BASE_NUTRIENTS.map((definition) => [definition.key, definition]));
const DERIVED_BY_KEY = new Map(DERIVED_NUTRIENTS.map((definition) => [definition.key, definition]));
const ORDER_BY_KEY = new Map(
  [...BASE_NUTRIENTS.map((definition) => definition.key), ...DERIVED_NUTRIENTS.map((definition) => definition.key)]
    .map((key, index) => [key, index]),
);

const MANAGED_KEYS: Record<NutrientFamily, Set<string>> = {
  cattle: new Set([
    'dry_matter_intake',
    'feed_intake',
    'energy_eke',
    'energy_oe_cattle',
    'crude_protein',
    'cp_pct_dm',
    'dig_protein_cattle',
    'dig_protein_cattle_pct_cp',
    'lysine',
    'methionine_cystine',
    'crude_fat',
    'crude_fiber',
    'starch',
    'starch_pct_dm',
    'sugar',
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
    'carotene',
    'vit_d3',
    'vit_e',
    'selenium',
    'ca_p_ratio',
  ]),
  swine: new Set([
    'feed_intake',
    'energy_oe_pig',
    'crude_protein',
    'crude_protein_pct',
    'dig_protein_pig',
    'lysine',
    'lysine_sid',
    'lysine_sid_pct',
    'methionine_cystine',
    'methionine_cystine_sid',
    'methionine_cystine_lys_ratio',
    'crude_fat',
    'crude_fiber',
    'starch',
    'sugar',
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
    'vit_d3',
    'vit_e',
    'selenium',
    'ca_p_ratio',
  ]),
  poultry: new Set([
    'feed_intake',
    'energy_oe_poultry',
    'crude_protein',
    'crude_protein_pct',
    'dig_protein_poultry',
    'lysine',
    'lysine_tid_pct',
    'methionine_cystine',
    'methionine_cystine_tid_pct',
    'methionine_cystine_lys_ratio',
    'crude_fat',
    'crude_fiber',
    'starch',
    'sugar',
    'calcium',
    'calcium_pct',
    'phosphorus',
    'sodium',
    'iron',
    'copper',
    'zinc',
    'manganese',
    'iodine',
    'vit_d3',
    'vit_e',
    'selenium',
    'ca_p_ratio',
  ]),
};

const GROUP_DISPLAY_BASIS_OVERRIDES: Partial<Record<string, Partial<Record<string, NutrientDisplayBasis>>>> = {
  swine_finisher: {
    crude_protein: 'per_kg_feed',
    lysine: 'per_kg_feed',
    lysine_sid: 'per_kg_feed',
    methionine_cystine: 'per_kg_feed',
    methionine_cystine_sid: 'per_kg_feed',
    phosphorus: 'per_kg_feed',
    calcium: 'per_kg_feed',
    sodium: 'per_kg_feed',
  },
};

const DISPLAY_BASIS_OVERRIDES: Partial<Record<string, Partial<Record<NutrientFamily, NutrientDisplayBasis>>>> = {
  starch_pct_dm: { cattle: 'percent_of_dm' },
  energy_oe_poultry: { poultry: 'per_kg_feed' },
  crude_protein: { poultry: 'percent_of_feed' },
  crude_protein_pct: { poultry: 'percent_of_feed' },
  lysine: { poultry: 'percent_of_feed' },
  lysine_sid_pct: { swine: 'percent_of_feed' },
  lysine_tid_pct: { poultry: 'percent_of_feed' },
  methionine_cystine: { poultry: 'percent_of_feed' },
  methionine_cystine_tid_pct: { poultry: 'percent_of_feed' },
  calcium: { poultry: 'percent_of_feed' },
  calcium_pct: { poultry: 'percent_of_feed' },
  phosphorus: { poultry: 'percent_of_feed' },
  sodium: { poultry: 'percent_of_feed' },
};

const NUTRIENT_ALIASES: Record<string, string> = {
  feed_units: 'koe',
  sugars: 'sugar',
  vitamin_d: 'vit_d3',
  vitamin_e: 'vit_e',
};

function prettifyMetricKey(key: string): string {
  return key
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function resolveNutrientLanguage(language?: string | null): NutrientLanguage {
  return language?.startsWith('en') ? 'en' : 'ru';
}

export function nutrientGroupFamily(groupId: string): NutrientFamily {
  if (groupId.startsWith('swine')) {
    return 'swine';
  }
  if (groupId.startsWith('poultry')) {
    return 'poultry';
  }
  return 'cattle';
}

export function resolveNutrientKey(key: string): string {
  return NUTRIENT_ALIASES[key] ?? key;
}

export function getNutrientMeta(key: string): NutrientMeta | undefined {
  const resolvedKey = resolveNutrientKey(key);
  return DERIVED_BY_KEY.get(resolvedKey) ?? BASE_BY_KEY.get(resolvedKey);
}

export function getNutrientLabels(key: string): { name_ru: string; name_en: string } | null {
  const meta = getNutrientMeta(key);
  if (!meta) {
    return null;
  }

  return {
    name_ru: meta.name_ru,
    name_en: meta.name_en,
  };
}

export function getNutrientLabel(key: string, language: NutrientLanguage): string {
  const meta = getNutrientMeta(key);
  if (!meta) {
    return prettifyMetricKey(resolveNutrientKey(key));
  }
  return language === 'en' ? meta.name_en : meta.name_ru;
}

export function getNutrientUnit(key: string, language: NutrientLanguage): string {
  const meta = getNutrientMeta(key);
  if (!meta) {
    return '';
  }
  return UNIT_LABELS[meta.unit_key][language];
}

export function getOrderedNutrientKeys(keys: Iterable<string>): string[] {
  return Array.from(new Set(keys)).sort((left, right) => {
    const leftOrder = ORDER_BY_KEY.get(resolveNutrientKey(left)) ?? Number.MAX_SAFE_INTEGER;
    const rightOrder = ORDER_BY_KEY.get(resolveNutrientKey(right)) ?? Number.MAX_SAFE_INTEGER;
    if (leftOrder !== rightOrder) {
      return leftOrder - rightOrder;
    }
    return left.localeCompare(right);
  });
}

export function getAllRegisteredNutrientKeys(): string[] {
  return getOrderedNutrientKeys(ORDER_BY_KEY.keys());
}

export function getNutrientCategory(key: string): NutrientCategory | undefined {
  return getNutrientMeta(key)?.category;
}

export function isManagedNutrientKey(groupId: string, key: string): boolean {
  const family = nutrientGroupFamily(groupId);
  return MANAGED_KEYS[family].has(resolveNutrientKey(key));
}

export function getManagedNormEntries(
  groupId: string,
  norms: Record<string, NormRange>,
): Array<[string, NormRange]> {
  const entries = new Map<string, NormRange>();

  Object.entries(norms).forEach(([key, norm]) => {
    const resolvedKey = resolveNutrientKey(key);
    if (!isManagedNutrientKey(groupId, resolvedKey) || entries.has(resolvedKey)) {
      return;
    }
    entries.set(resolvedKey, norm);
  });

  return getOrderedNutrientKeys(entries.keys()).map((key) => [key, entries.get(key)!]);
}

export function getNutrientDisplayBasis(groupId: string, key: string): NutrientDisplayBasis {
  const family = nutrientGroupFamily(groupId);
  const resolvedKey = resolveNutrientKey(key);
  const exactGroupBasis = GROUP_DISPLAY_BASIS_OVERRIDES[groupId]?.[resolvedKey];
  if (exactGroupBasis) {
    return exactGroupBasis;
  }

  const familyOverride = DISPLAY_BASIS_OVERRIDES[resolvedKey]?.[family];
  if (familyOverride) {
    return familyOverride;
  }

  const meta = getNutrientMeta(resolvedKey);
  if (!meta) {
    return 'absolute';
  }

  if (
    family === 'poultry' &&
    meta.category !== 'energy' &&
    meta.category !== 'ratio' &&
    meta.category !== 'intake'
  ) {
    return 'per_kg_feed';
  }

  if (
    family === 'swine' &&
    !groupId.includes('sow') &&
    meta.category !== 'energy' &&
    meta.category !== 'ratio' &&
    meta.category !== 'intake'
  ) {
    return 'per_kg_feed';
  }

  return 'absolute';
}

export function getNutrientDisplayUnit(
  groupId: string,
  key: string,
  language: NutrientLanguage,
  fallbackUnit?: string,
): string {
  const unit = fallbackUnit ?? getNutrientUnit(key, language);
  switch (getNutrientDisplayBasis(groupId, key)) {
    case 'per_kg_feed':
      return unit ? `${unit}/${language === 'en' ? 'kg' : 'кг'}` : language === 'en' ? 'per kg feed' : 'на кг корма';
    case 'percent_of_feed':
      return '%';
    case 'percent_of_dm':
      return language === 'en' ? '% DM' : '% СВ';
    default:
      return unit;
  }
}
