import type { Feed } from './feed';
import type { AlternativeRationSolution } from './optimization';

export type OptimizeMode =
  | 'minimize_cost'
  | 'balance'
  | 'single_pass'
  | 'tiered'
  | 'fixed'
  | 'repair';
export type SolveIntent = 'selected_only' | 'complete_from_library' | 'build_from_library';
export type RationState = 'empty' | 'sparse' | 'structured';
export type RelaxedConstraintType = 'min' | 'max' | 'target';

export interface OptimizeAnimalPropertiesPayload {
  species?: string;
  production_type?: string;
  breed?: string;
  sex?: 'male' | 'female' | 'mixed';
  live_weight_kg?: number;
  age_from_days?: number;
  age_to_days?: number;
  milk_yield_kg?: number;
  milk_fat_pct?: number;
  daily_gain_g?: number;
  egg_production_per_year?: number;
  litter_size?: number;
  reproductive_stage?: 'gestation' | 'lactation';
}

export interface Ration {
  id: number;
  name: string;
  animal_group_id?: string;
  animal_count: number;
  description?: string;
  status: 'draft' | 'active' | 'archived';
  created_at?: string;
  updated_at?: string;
}

export interface RationItem {
  id: number;
  ration_id: number;
  feed_id: number;
  feed?: Feed;
  amount_kg: number;
  is_locked: boolean;
  sort_order: number;
}

export interface RationFull {
  ration: Ration;
  items: RationItem[];
}

export interface OptimizedItem {
  feed_id: number;
  feed_name: string;
  amount_kg: number;
  dm_kg: number;
  cost_per_day: number;
}

export interface DietSolution {
  items: OptimizedItem[];
  nutrient_summary: NutrientSummary;
  cost_per_day: number;
  optimization_status: 'Optimal' | 'Feasible' | 'Infeasible' | 'Unbounded' | 'Error';
  warnings: NutritionWarning[];
  recommendations: FeedRecommendation[];
  applied_strategy: string;
  auto_populated: boolean;
  solve_intent?: SolveIntent;
  ration_state?: RationState;
  workflow_notes?: string[];
  best_achievable?: boolean;
  relaxed_targets?: RelaxedTarget[];
  auto_added_feeds?: AutoAddedFeed[];
  alternatives?: AlternativeRationSolution[];
}

export interface RelaxedTarget {
  key: string;
  constraint_type: RelaxedConstraintType;
  target: number;
  actual: number;
  delta: number;
}

export interface NutrientSummary {
  total_weight_kg: number;
  total_dm_kg: number;
  energy_eke: number;
  energy_oe_cattle: number;
  energy_oe_pig: number;
  energy_oe_poultry: number;
  crude_protein: number;
  dig_protein_cattle: number;
  dig_protein_pig: number;
  dig_protein_poultry: number;
  lysine: number;
  methionine_cystine: number;
  crude_fat: number;
  crude_fiber: number;
  starch: number;
  sugar: number;
  calcium: number;
  phosphorus: number;
  ca_p_ratio: number;
  magnesium: number;
  potassium: number;
  sodium: number;
  sulfur: number;
  iron: number;
  copper: number;
  zinc: number;
  manganese: number;
  cobalt: number;
  iodine: number;
  vit_d3: number;
  vit_e: number;
  carotene: number;
  dm_pct: number;
  cp_pct_dm: number;
  dig_protein_cattle_pct_cp: number;
  starch_pct_dm: number;
}

export interface NutritionWarning {
  type: string;
  message: string;
  severity: 'info' | 'warning' | 'error';
}

export interface FeedRecommendation {
  feed_id: number;
  feed_name: string;
  reason: string;
  suggested_amount_kg: number;
  category: string;
  priority: number;
}

export interface AutoAddedFeed {
  feed_id: number;
  feed_name: string;
  amount_kg: number;
  reasons: string[];
}

export interface AutoPopulateItem {
  feed: Feed;
  amount_kg: number;
  group: string;
  reason: string;
}

export interface AutoPopulatePlan {
  items: AutoPopulateItem[];
  notes: string[];
}

export interface ScreeningReport {
  can_meet_reference: boolean;
  limiting_nutrients: string[];
  recommendations: FeedRecommendation[];
}

export interface OptimizeRationRequest {
  mode?: OptimizeMode;
  intent?: SolveIntent;
  norms?: Record<string, { min?: number; target?: number; max?: number }>;
  norm_preset_id?: string | null;
  animal_properties?: OptimizeAnimalPropertiesPayload;
  available_feed_ids?: number[];
}

export interface EconomicAnalysis {
  feed_cost_per_day: number;
  feed_cost_per_month: number;
  feed_cost_per_year: number;
  feed_cost_per_unit: number;
  cost_by_category: CategoryCost[];
  cost_per_animal_day: number;
  animal_count: number;
  total_daily_cost: number;
}

export interface CategoryCost {
  category: string;
  cost_per_day: number;
  percentage: number;
}
