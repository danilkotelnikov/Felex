export type NutrientCategoryId =
  | 'general'
  | 'energy'
  | 'protein'
  | 'amino_acids'
  | 'fiber_carbs'
  | 'fats'
  | 'macrominerals'
  | 'trace_minerals'
  | 'vitamins'
  | 'ratios';

export interface NutrientCategoryDefinition {
  id: NutrientCategoryId;
  titleKey: string;
  order: number;
  defaultOpen?: boolean;
}

export const NUTRIENT_CATEGORIES: readonly NutrientCategoryDefinition[] = [
  { id: 'general', titleKey: 'nutrients.categories.general', order: 1, defaultOpen: true },
  { id: 'energy', titleKey: 'nutrients.categories.energy', order: 2, defaultOpen: true },
  { id: 'protein', titleKey: 'nutrients.categories.protein', order: 3, defaultOpen: true },
  { id: 'amino_acids', titleKey: 'nutrients.categories.amino_acids', order: 4, defaultOpen: true },
  { id: 'fiber_carbs', titleKey: 'nutrients.categories.fiber_carbs', order: 5, defaultOpen: true },
  { id: 'fats', titleKey: 'nutrients.categories.fats', order: 6 },
  { id: 'macrominerals', titleKey: 'nutrients.categories.macrominerals', order: 7, defaultOpen: true },
  { id: 'trace_minerals', titleKey: 'nutrients.categories.trace_minerals', order: 8 },
  { id: 'vitamins', titleKey: 'nutrients.categories.vitamins', order: 9 },
  { id: 'ratios', titleKey: 'nutrients.categories.ratios', order: 10 },
] as const;

const NUTRIENT_TO_CATEGORY: Record<string, NutrientCategoryId> = {
  dry_matter: 'general',
  dry_matter_intake: 'general',
  feed_intake: 'general',
  dm_pct: 'general',

  feed_units: 'energy',
  energy_eke: 'energy',
  metabolizable_energy: 'energy',
  energy_oe_cattle: 'energy',
  energy_oe_pig: 'energy',
  energy_oe_poultry: 'energy',

  crude_protein: 'protein',
  crude_protein_pct: 'protein',
  cp_pct_dm: 'protein',
  digestible_protein: 'protein',
  dig_protein_cattle: 'protein',
  dig_protein_pig: 'protein',
  dig_protein_poultry: 'protein',
  dig_protein_cattle_pct_cp: 'protein',

  lysine: 'amino_acids',
  lysine_sid: 'amino_acids',
  lysine_sid_pct: 'amino_acids',
  lysine_tid_pct: 'amino_acids',
  methionine_cystine: 'amino_acids',
  methionine_cystine_sid: 'amino_acids',
  methionine_cystine_tid_pct: 'amino_acids',

  crude_fiber: 'fiber_carbs',
  starch: 'fiber_carbs',
  starch_pct_dm: 'fiber_carbs',
  sugar: 'fiber_carbs',
  sugars: 'fiber_carbs',

  crude_fat: 'fats',

  calcium: 'macrominerals',
  calcium_pct: 'macrominerals',
  phosphorus: 'macrominerals',
  magnesium: 'macrominerals',
  potassium: 'macrominerals',
  sodium: 'macrominerals',
  sulfur: 'macrominerals',

  iron: 'trace_minerals',
  copper: 'trace_minerals',
  zinc: 'trace_minerals',
  manganese: 'trace_minerals',
  cobalt: 'trace_minerals',
  iodine: 'trace_minerals',

  carotene: 'vitamins',
  vit_d3: 'vitamins',
  vit_e: 'vitamins',

  ca_p_ratio: 'ratios',
  methionine_cystine_lys_ratio: 'ratios',
};

export function getNutrientCategoryId(nutrientKey: string): NutrientCategoryId {
  return NUTRIENT_TO_CATEGORY[nutrientKey] ?? 'general';
}
