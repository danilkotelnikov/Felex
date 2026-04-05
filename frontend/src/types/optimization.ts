// frontend/src/types/optimization.ts
import type { NutrientSummary, NutritionWarning, OptimizedItem } from './ration';

export type SolutionStatus = 'Optimal' | 'Feasible' | 'Infeasible' | 'Unbounded' | 'Error';

export interface AlternativeRationSolution {
  id: string;
  label: string;
  feeds: OptimizedItem[];
  nutrients: NutrientSummary;
  adequacy_score: number;
  cost: number;
  tags: string[];
  optimization_status: SolutionStatus;
  applied_strategy: string;
  warnings: NutritionWarning[];
}

export interface OptimizationComparison {
  cost_range: [number, number];
  score_range: [number, number];
  common_feeds: string[];
  differentiators: string[];
}

export interface OptimizationResult {
  primary: AlternativeRationSolution;
  alternatives: AlternativeRationSolution[];
  comparison: OptimizationComparison;
}

export type NutrientStatus = 'ok' | 'near' | 'out';

export interface NutrientStatusInfo {
  key: string;
  value: number;
  status: NutrientStatus;
  abbr: string;
  unit: string;
}
