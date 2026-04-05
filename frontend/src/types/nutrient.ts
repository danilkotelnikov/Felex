export interface NutrientDef {
  key: string;
  name_ru: string;
  name_en: string;
  unit: string;
  category: 'energy' | 'protein' | 'fiber' | 'mineral' | 'vitamin';
}

export type NutrientStatus = 'ok' | 'low' | 'high' | 'critical';

export interface NutrientRowData {
  key: string;
  name: string;
  actual: number;
  norm_min?: number;
  norm_max?: number;
  norm_target?: number;
  unit: string;
  status: NutrientStatus;
  percent_of_norm: number;
}

export interface AnimalNorm {
  id: string;
  species: string;
  production_type?: string;
  breed_group?: string;
  sex?: string;
  age_from_days?: number;
  age_to_days?: number;
  weight_from_kg?: number;
  weight_to_kg?: number;
  milk_yield_kg?: number;
  milk_fat_pct?: number;
  milk_protein_pct?: number;
  daily_gain_g?: number;
  nutrients_min: Record<string, number>;
  nutrients_max: Record<string, number>;
  nutrients_target: Record<string, number>;
  feed_intake_min?: number;
  feed_intake_max?: number;
  notes?: string;
  source?: string;
}

export interface NormMethodologyMetric {
  key: string;
  unit: string;
  reference_value?: number;
  current_value?: number;
}

export interface NormMethodologyFactor {
  key: string;
  value: number;
}

export interface NormMethodology {
  key: string;
  reference_group_id: string;
  dynamic: boolean;
  source_refs: string[];
  driver_metrics: NormMethodologyMetric[];
  derived_metrics: NormMethodologyMetric[];
  scaling_factors: NormMethodologyFactor[];
}

export interface NormRange {
  min?: number;
  max?: number;
  target?: number;
}

export interface NormPreset {
  id: string;
  groupId: string;
  label_ru: string;
  label_en: string;
  params: {
    weight?: number;
    milkYield?: number;
    fatPct?: number;
    dailyGain?: number;
    eggProduction?: number;
    age?: string;
  };
  norms: Record<string, NormRange>;
}

export const NUTRIENT_DEFS: NutrientDef[] = [
  { key: 'energy_eke', name_ru: 'ЭКЕ', name_en: 'Energy units', unit: 'ЭКЕ', category: 'energy' },
  { key: 'energy_oe_cattle', name_ru: 'ОЭ КРС', name_en: 'ME cattle', unit: 'МДж', category: 'energy' },
  { key: 'energy_oe_pig', name_ru: 'ОЭ свиней', name_en: 'ME swine', unit: 'МДж', category: 'energy' },
  { key: 'energy_oe_poultry', name_ru: 'ОЭ птицы', name_en: 'ME poultry', unit: 'МДж', category: 'energy' },
  { key: 'crude_protein', name_ru: 'Сырой протеин', name_en: 'Crude protein', unit: 'г', category: 'protein' },
  { key: 'dig_protein_cattle', name_ru: 'Переваримый протеин, КРС', name_en: 'Digestible protein, cattle', unit: 'г', category: 'protein' },
  { key: 'dig_protein_pig', name_ru: 'Переваримый протеин, свиньи', name_en: 'Digestible protein, swine', unit: 'г', category: 'protein' },
  { key: 'dig_protein_poultry', name_ru: 'Переваримый протеин, птица', name_en: 'Digestible protein, poultry', unit: 'г', category: 'protein' },
  { key: 'lysine', name_ru: 'Лизин', name_en: 'Lysine', unit: 'г', category: 'protein' },
  { key: 'methionine_cystine', name_ru: 'Метионин + цистин', name_en: 'Methionine + cystine', unit: 'г', category: 'protein' },
  { key: 'crude_fiber', name_ru: 'Сырая клетчатка', name_en: 'Crude fiber', unit: 'г', category: 'fiber' },
  { key: 'crude_fat', name_ru: 'Сырой жир', name_en: 'Crude fat', unit: 'г', category: 'fiber' },
  { key: 'starch', name_ru: 'Крахмал', name_en: 'Starch', unit: 'г', category: 'fiber' },
  { key: 'sugar', name_ru: 'Сахар', name_en: 'Sugar', unit: 'г', category: 'fiber' },
  { key: 'calcium', name_ru: 'Кальций', name_en: 'Calcium', unit: 'г', category: 'mineral' },
  { key: 'phosphorus', name_ru: 'Фосфор', name_en: 'Phosphorus', unit: 'г', category: 'mineral' },
  { key: 'magnesium', name_ru: 'Магний', name_en: 'Magnesium', unit: 'г', category: 'mineral' },
  { key: 'potassium', name_ru: 'Калий', name_en: 'Potassium', unit: 'г', category: 'mineral' },
  { key: 'sodium', name_ru: 'Натрий', name_en: 'Sodium', unit: 'г', category: 'mineral' },
  { key: 'sulfur', name_ru: 'Сера', name_en: 'Sulfur', unit: 'г', category: 'mineral' },
  { key: 'iron', name_ru: 'Железо', name_en: 'Iron', unit: 'мг', category: 'mineral' },
  { key: 'copper', name_ru: 'Медь', name_en: 'Copper', unit: 'мг', category: 'mineral' },
  { key: 'zinc', name_ru: 'Цинк', name_en: 'Zinc', unit: 'мг', category: 'mineral' },
  { key: 'manganese', name_ru: 'Марганец', name_en: 'Manganese', unit: 'мг', category: 'mineral' },
  { key: 'cobalt', name_ru: 'Кобальт', name_en: 'Cobalt', unit: 'мг', category: 'mineral' },
  { key: 'iodine', name_ru: 'Йод', name_en: 'Iodine', unit: 'мг', category: 'mineral' },
  { key: 'carotene', name_ru: 'Каротин', name_en: 'Carotene', unit: 'мг', category: 'vitamin' },
  { key: 'vit_d3', name_ru: 'Витамин D3', name_en: 'Vitamin D3', unit: 'МЕ', category: 'vitamin' },
  { key: 'vit_e', name_ru: 'Витамин E', name_en: 'Vitamin E', unit: 'мг', category: 'vitamin' },
  { key: 'selenium', name_ru: 'Селен', name_en: 'Selenium', unit: 'мг', category: 'mineral' },
];

export const NORMS_BY_GROUP: Record<string, Record<string, NormRange>> = {
  cattle_dairy: {
    energy_eke: { min: 20.5, target: 21.5 },
    energy_oe_cattle: { min: 215, target: 225 },
    crude_protein: { min: 3050, target: 3200 },
    dig_protein_cattle: { min: 2050, target: 2200 },
    lysine: { min: 150 },
    methionine_cystine: { min: 80 },
    crude_fiber: { min: 5500, max: 7500 },
    crude_fat: { max: 1200 },
    starch: { max: 5500 },
    sugar: { min: 1200, max: 2500 },
    calcium: { min: 120, target: 135 },
    phosphorus: { min: 80, target: 90 },
    magnesium: { min: 30, target: 40 },
    potassium: { min: 120, target: 150 },
    sodium: { min: 25, target: 35 },
    sulfur: { min: 30, target: 40 },
    iron: { min: 1000, target: 1500 },
    copper: { min: 150, target: 200 },
    zinc: { min: 600, target: 900 },
    manganese: { min: 600, target: 900 },
    cobalt: { min: 5, target: 8 },
    iodine: { min: 8, target: 12 },
    vit_d3: { min: 20000 },
    vit_e: { min: 500 },
    selenium: { min: 3, target: 4, max: 5 },
  },
  cattle_beef: {
    energy_eke: { min: 10, target: 12 },
    energy_oe_cattle: { min: 105, target: 126 },
    crude_protein: { min: 1100, target: 1300 },
    dig_protein_cattle: { min: 750, target: 900 },
    crude_fiber: { min: 3000, max: 5000 },
    calcium: { min: 45, target: 60 },
    phosphorus: { min: 30, target: 40 },
    magnesium: { min: 15, target: 20 },
    sodium: { min: 12, target: 18 },
    sulfur: { min: 15, target: 20 },
    vit_d3: { min: 7000 },
    vit_e: { min: 300 },
    selenium: { min: 2, target: 2.5, max: 3 },
  },
  swine_finisher: {
    energy_eke: { min: 3.2, target: 3.6 },
    energy_oe_pig: { min: 33, target: 37 },
    crude_protein: { min: 250, target: 300 },
    lysine: { min: 14, target: 17 },
    methionine_cystine: { min: 8.5, target: 10.2 },
    crude_fiber: { max: 200 },
    calcium: { min: 10, target: 14 },
    phosphorus: { min: 8, target: 10 },
    sodium: { min: 3, target: 5 },
    iron: { min: 160, target: 200 },
    copper: { min: 20, target: 30 },
    zinc: { min: 100, target: 150 },
    manganese: { min: 80, target: 100 },
    iodine: { min: 0.3, target: 0.5 },
    vit_d3: { min: 1000 },
    vit_e: { min: 80 },
    selenium: { min: 0.2, target: 0.25, max: 0.3 },
  },
  swine_sow: {
    energy_eke: { min: 3.5, target: 4.0 },
    energy_oe_pig: { min: 36, target: 41 },
    crude_protein: { min: 280, target: 330 },
    lysine: { min: 17, target: 22 },
    methionine_cystine: { min: 10, target: 13 },
    crude_fiber: { max: 300 },
    calcium: { min: 18, target: 22 },
    phosphorus: { min: 12, target: 16 },
    sodium: { min: 4, target: 6 },
    iron: { min: 200, target: 250 },
    copper: { min: 25, target: 35 },
    zinc: { min: 130, target: 180 },
    manganese: { min: 80, target: 120 },
    iodine: { min: 0.3, target: 0.5 },
    vit_d3: { min: 1500 },
    vit_e: { min: 120 },
    selenium: { min: 0.2, target: 0.25, max: 0.3 },
  },
  poultry_broiler: {
    energy_eke: { min: 0.32, target: 0.34 },
    energy_oe_poultry: { min: 12.5, target: 13.2 },
    crude_protein: { min: 22, target: 24 },
    lysine: { min: 1.2, target: 1.35 },
    methionine_cystine: { min: 0.9, target: 1.05 },
    crude_fiber: { max: 4.5 },
    crude_fat: { min: 3, max: 8 },
    calcium: { min: 0.9, target: 1.1 },
    phosphorus: { min: 0.45, target: 0.55 },
    sodium: { min: 0.15, target: 0.2 },
    iron: { min: 80 },
    copper: { min: 8 },
    zinc: { min: 40 },
    manganese: { min: 60 },
    iodine: { min: 0.35 },
    vit_d3: { min: 500 },
    vit_e: { min: 30 },
    selenium: { min: 0.15, target: 0.2, max: 0.25 },
  },
  poultry_layer: {
    energy_eke: { min: 0.27, target: 0.29 },
    energy_oe_poultry: { min: 11.3, target: 11.9 },
    crude_protein: { min: 17, target: 18 },
    lysine: { min: 0.75, target: 0.85 },
    methionine_cystine: { min: 0.62, target: 0.73 },
    crude_fiber: { max: 6 },
    crude_fat: { min: 2.5, max: 6 },
    calcium: { min: 3.5, target: 4.0, max: 4.5 },
    phosphorus: { min: 0.35, target: 0.45 },
    sodium: { min: 0.12, target: 0.18 },
    iron: { min: 50 },
    copper: { min: 5 },
    zinc: { min: 50 },
    manganese: { min: 60 },
    iodine: { min: 0.35 },
    vit_d3: { min: 750 },
    vit_e: { min: 20 },
  },
};

function withNormOverrides(
  base: Record<string, NormRange>,
  overrides: Record<string, NormRange>,
): Record<string, NormRange> {
  return { ...base, ...overrides };
}

export const NORM_PRESETS: NormPreset[] = [
  {
    id: 'cattle_dairy_20',
    groupId: 'cattle_dairy',
    label_ru: 'Дойные коровы, 20 кг молока',
    label_en: 'Dairy cows, 20 kg milk',
    params: { weight: 550, milkYield: 20, fatPct: 3.8 },
    norms: withNormOverrides(NORMS_BY_GROUP.cattle_dairy, {
      energy_eke: { min: 16.5, target: 17.5 },
      energy_oe_cattle: { min: 173, target: 184 },
      crude_protein: { min: 2400, target: 2600 },
      dig_protein_cattle: { min: 1600, target: 1800 },
      crude_fiber: { min: 4800, max: 6800 },
      calcium: { min: 95, target: 110 },
      phosphorus: { min: 65, target: 75 },
      magnesium: { min: 25, target: 32 },
      potassium: { min: 100, target: 130 },
      sodium: { min: 20, target: 28 },
      sulfur: { min: 25, target: 32 },
      iron: { min: 800, target: 1200 },
      copper: { min: 120, target: 170 },
      zinc: { min: 500, target: 750 },
      manganese: { min: 500, target: 750 },
      cobalt: { min: 4, target: 7 },
      iodine: { min: 6, target: 10 },
      lysine: { min: 120 },
      methionine_cystine: { min: 65 },
      vit_d3: { min: 15000 },
      vit_e: { min: 400 },
    }),
  },
  {
    id: 'cattle_dairy_25',
    groupId: 'cattle_dairy',
    label_ru: 'Дойные коровы, 25 кг молока',
    label_en: 'Dairy cows, 25 kg milk',
    params: { weight: 600, milkYield: 25, fatPct: 3.7 },
    norms: withNormOverrides(NORMS_BY_GROUP.cattle_dairy, {
      energy_eke: { min: 18.5, target: 19.5 },
      energy_oe_cattle: { min: 194, target: 205 },
      crude_protein: { min: 2750, target: 2950 },
      dig_protein_cattle: { min: 1850, target: 2000 },
      crude_fiber: { min: 5200, max: 7200 },
      calcium: { min: 108, target: 122 },
      phosphorus: { min: 72, target: 82 },
      magnesium: { min: 28, target: 36 },
      potassium: { min: 110, target: 140 },
      sodium: { min: 22, target: 32 },
      sulfur: { min: 28, target: 36 },
      iron: { min: 900, target: 1350 },
      copper: { min: 135, target: 185 },
      zinc: { min: 550, target: 820 },
      manganese: { min: 550, target: 820 },
      cobalt: { min: 4.5, target: 7.5 },
      iodine: { min: 7, target: 11 },
      lysine: { min: 135 },
      methionine_cystine: { min: 72 },
      vit_d3: { min: 17500 },
      vit_e: { min: 450 },
    }),
  },
  {
    id: 'cattle_dairy_30',
    groupId: 'cattle_dairy',
    label_ru: 'Дойные коровы, 30 кг молока',
    label_en: 'Dairy cows, 30 kg milk',
    params: { weight: 600, milkYield: 30, fatPct: 3.7 },
    norms: NORMS_BY_GROUP.cattle_dairy,
  },
  {
    id: 'cattle_dairy_35',
    groupId: 'cattle_dairy',
    label_ru: 'Дойные коровы, 35 кг молока',
    label_en: 'Dairy cows, 35 kg milk',
    params: { weight: 650, milkYield: 35, fatPct: 3.6 },
    norms: withNormOverrides(NORMS_BY_GROUP.cattle_dairy, {
      energy_eke: { min: 23.5, target: 25.0 },
      energy_oe_cattle: { min: 247, target: 263 },
      crude_protein: { min: 3500, target: 3750 },
      dig_protein_cattle: { min: 2350, target: 2550 },
      crude_fiber: { min: 5800, max: 7800 },
      crude_fat: { max: 1400 },
      calcium: { min: 140, target: 160 },
      phosphorus: { min: 95, target: 108 },
      magnesium: { min: 35, target: 45 },
      potassium: { min: 140, target: 175 },
      sodium: { min: 30, target: 42 },
      sulfur: { min: 35, target: 45 },
      iron: { min: 1200, target: 1800 },
      copper: { min: 175, target: 240 },
      zinc: { min: 700, target: 1050 },
      manganese: { min: 700, target: 1050 },
      cobalt: { min: 6, target: 10 },
      iodine: { min: 10, target: 15 },
      lysine: { min: 175 },
      methionine_cystine: { min: 95 },
      vit_d3: { min: 25000 },
      vit_e: { min: 600 },
    }),
  },
  {
    id: 'cattle_beef_300',
    groupId: 'cattle_beef',
    label_ru: 'Откорм КРС, 300-400 кг',
    label_en: 'Beef finishing, 300-400 kg',
    params: { weight: 350, dailyGain: 1100 },
    norms: withNormOverrides(NORMS_BY_GROUP.cattle_beef, {
      energy_eke: { min: 8.5, target: 10 },
      energy_oe_cattle: { min: 89, target: 105 },
      crude_protein: { min: 950, target: 1100 },
      dig_protein_cattle: { min: 650, target: 770 },
      crude_fiber: { min: 2500, max: 4200 },
      calcium: { min: 38, target: 50 },
      phosphorus: { min: 25, target: 33 },
      magnesium: { min: 12, target: 17 },
      sodium: { min: 10, target: 15 },
      sulfur: { min: 12, target: 17 },
      vit_d3: { min: 6000 },
      vit_e: { min: 250 },
    }),
  },
  {
    id: 'cattle_beef_400',
    groupId: 'cattle_beef',
    label_ru: 'Откорм КРС, 400-500 кг',
    label_en: 'Beef finishing, 400-500 kg',
    params: { weight: 450, dailyGain: 1000 },
    norms: NORMS_BY_GROUP.cattle_beef,
  },
  {
    id: 'cattle_beef_500',
    groupId: 'cattle_beef',
    label_ru: 'Откорм КРС, 500+ кг',
    label_en: 'Beef finishing, 500+ kg',
    params: { weight: 550, dailyGain: 900 },
    norms: withNormOverrides(NORMS_BY_GROUP.cattle_beef, {
      energy_eke: { min: 11.5, target: 13.5 },
      energy_oe_cattle: { min: 121, target: 142 },
      crude_protein: { min: 1250, target: 1450 },
      dig_protein_cattle: { min: 850, target: 1000 },
      crude_fiber: { min: 3500, max: 5500 },
      calcium: { min: 52, target: 68 },
      phosphorus: { min: 35, target: 46 },
      magnesium: { min: 18, target: 24 },
      sodium: { min: 14, target: 20 },
      sulfur: { min: 17, target: 23 },
      vit_d3: { min: 8000 },
      vit_e: { min: 350 },
    }),
  },
  {
    id: 'swine_starter',
    groupId: 'swine_finisher',
    label_ru: 'Поросята на доращивании, 20-30 кг',
    label_en: 'Piglet grower, 20-30 kg',
    params: { weight: 25, dailyGain: 600, age: '2-3 мес.' },
    norms: withNormOverrides(NORMS_BY_GROUP.swine_finisher, {
      energy_eke: { min: 2.0, target: 2.3 },
      crude_protein: { min: 200, target: 230 },
      lysine: { min: 12, target: 14 },
      methionine_cystine: { min: 7.5, target: 9 },
      crude_fiber: { max: 100 },
      calcium: { min: 8, target: 10 },
      phosphorus: { min: 6, target: 8 },
      sodium: { min: 2, target: 3 },
      iron: { min: 180, target: 220 },
      copper: { min: 25, target: 35 },
      zinc: { min: 120, target: 170 },
      manganese: { min: 90, target: 120 },
      vit_d3: { min: 800 },
      vit_e: { min: 60 },
    }),
  },
  {
    id: 'swine_grower',
    groupId: 'swine_finisher',
    label_ru: 'Откорм свиней, гроуер, 30-60 кг',
    label_en: 'Pig grower, 30-60 kg',
    params: { weight: 45, dailyGain: 750, age: '3-5 мес.' },
    norms: withNormOverrides(NORMS_BY_GROUP.swine_finisher, {
      energy_eke: { min: 2.8, target: 3.2 },
      crude_protein: { min: 230, target: 270 },
      lysine: { min: 13, target: 15.5 },
      methionine_cystine: { min: 8.0, target: 9.5 },
      crude_fiber: { max: 160 },
      calcium: { min: 9, target: 12 },
      phosphorus: { min: 7, target: 9 },
      sodium: { min: 2.5, target: 4 },
      iron: { min: 170, target: 210 },
      copper: { min: 22, target: 32 },
      zinc: { min: 110, target: 160 },
      manganese: { min: 85, target: 110 },
      vit_d3: { min: 900 },
      vit_e: { min: 70 },
    }),
  },
  {
    id: 'swine_finisher_preset',
    groupId: 'swine_finisher',
    label_ru: 'Откорм свиней, финишер, 60-110 кг',
    label_en: 'Pig finisher, 60-110 kg',
    params: { weight: 85, dailyGain: 900, age: '5-7 мес.' },
    norms: NORMS_BY_GROUP.swine_finisher,
  },
  {
    id: 'swine_sow_gestation',
    groupId: 'swine_sow',
    label_ru: 'Свиноматки, супоросные',
    label_en: 'Gestating sows',
    params: { weight: 200, age: 'супоросность' },
    norms: withNormOverrides(NORMS_BY_GROUP.swine_sow, {
      energy_eke: { min: 3.0, target: 3.4 },
      crude_protein: { min: 240, target: 280 },
      lysine: { min: 12, target: 16 },
      methionine_cystine: { min: 7.5, target: 9.5 },
      calcium: { min: 15, target: 18 },
      phosphorus: { min: 10, target: 13 },
      sodium: { min: 3.5, target: 5 },
      iron: { min: 180 },
      vit_d3: { min: 1200 },
      vit_e: { min: 100 },
    }),
  },
  {
    id: 'swine_sow_lactation',
    groupId: 'swine_sow',
    label_ru: 'Свиноматки, подсосные',
    label_en: 'Lactating sows',
    params: { weight: 210, age: 'лактация' },
    norms: NORMS_BY_GROUP.swine_sow,
  },
  {
    id: 'poultry_broiler_starter',
    groupId: 'poultry_broiler',
    label_ru: 'Бройлеры, стартер (0-10 дн.)',
    label_en: 'Broiler starter (0-10 d)',
    params: { dailyGain: 30, age: '0-10 дн.' },
    norms: withNormOverrides(NORMS_BY_GROUP.poultry_broiler, {
      energy_eke: { min: 0.30, target: 0.32 },
      crude_protein: { min: 23, target: 25 },
      lysine: { min: 1.3, target: 1.45 },
      methionine_cystine: { min: 1.0, target: 1.15 },
      calcium: { min: 1.0, target: 1.2 },
      phosphorus: { min: 0.5, target: 0.6 },
      sodium: { min: 0.16, target: 0.22 },
      vit_d3: { min: 600 },
      vit_e: { min: 40 },
    }),
  },
  {
    id: 'poultry_broiler_grower',
    groupId: 'poultry_broiler',
    label_ru: 'Бройлеры, гроуер (11-24 дн.)',
    label_en: 'Broiler grower (11-24 d)',
    params: { dailyGain: 55, age: '11-24 дн.' },
    norms: NORMS_BY_GROUP.poultry_broiler,
  },
  {
    id: 'poultry_broiler_finisher',
    groupId: 'poultry_broiler',
    label_ru: 'Бройлеры, финишер (25-42 дн.)',
    label_en: 'Broiler finisher (25-42 d)',
    params: { dailyGain: 70, age: '25-42 дн.' },
    norms: withNormOverrides(NORMS_BY_GROUP.poultry_broiler, {
      energy_eke: { min: 0.33, target: 0.35 },
      crude_protein: { min: 20, target: 22 },
      lysine: { min: 1.05, target: 1.2 },
      methionine_cystine: { min: 0.8, target: 0.95 },
      calcium: { min: 0.85, target: 1.0 },
      phosphorus: { min: 0.4, target: 0.5 },
      sodium: { min: 0.13, target: 0.18 },
      vit_d3: { min: 400 },
      vit_e: { min: 25 },
    }),
  },
  {
    id: 'poultry_layer_phase1',
    groupId: 'poultry_layer',
    label_ru: 'Куры-несушки, фаза 1 (20-45 нед.)',
    label_en: 'Layers phase 1 (20-45 wk)',
    params: { eggProduction: 95, age: '20-45 нед.' },
    norms: NORMS_BY_GROUP.poultry_layer,
  },
  {
    id: 'poultry_layer_phase2',
    groupId: 'poultry_layer',
    label_ru: 'Куры-несушки, фаза 2 (45+ нед.)',
    label_en: 'Layers phase 2 (45+ wk)',
    params: { eggProduction: 80, age: '45+ нед.' },
    norms: withNormOverrides(NORMS_BY_GROUP.poultry_layer, {
      energy_eke: { min: 0.25, target: 0.27 },
      crude_protein: { min: 15.5, target: 16.5 },
      lysine: { min: 0.68, target: 0.78 },
      methionine_cystine: { min: 0.55, target: 0.66 },
      calcium: { min: 3.8, target: 4.2, max: 4.8 },
      phosphorus: { min: 0.32, target: 0.42 },
      sodium: { min: 0.12, target: 0.17 },
      vit_d3: { min: 650 },
      vit_e: { min: 18 },
    }),
  },
];

export function getPresetsForGroup(groupId: string): NormPreset[] {
  return NORM_PRESETS.filter((preset) => preset.groupId === groupId);
}

export function getBaseNormsForGroupId(groupId: string): Record<string, NormRange> {
  return NORMS_BY_GROUP[groupId] ?? {};
}

export function getNormsForGroup(species: string, productionType: string): Record<string, NormRange> {
  const groupMap: Record<string, string> = {
    cattle_dairy: 'cattle_dairy',
    cattle_beef: 'cattle_beef',
    swine_fattening: 'swine_finisher',
    swine_breeding: 'swine_sow',
    poultry_broiler: 'poultry_broiler',
    poultry_layer: 'poultry_layer',
  };

  const key = `${species}_${productionType}`;
  return NORMS_BY_GROUP[groupMap[key] ?? 'cattle_dairy'] ?? NORMS_BY_GROUP.cattle_dairy;
}

export function getNutrientStatus(
  actual: number,
  min?: number,
  max?: number,
  _target?: number,
): NutrientStatus {
  if (min !== undefined && actual < min * 0.8) return 'critical';
  if (min !== undefined && actual < min) return 'low';
  if (max !== undefined && actual > max * 1.2) return 'critical';
  if (max !== undefined && actual > max) return 'high';
  return 'ok';
}

export function getPercentOfNorm(actual: number, target?: number, min?: number): number {
  const norm = target ?? min ?? 0;
  if (norm === 0) return 0;
  return (actual / norm) * 100;
}
