import type { NutrientStatus } from '@/types/nutrient';

export type { NutrientStatus } from '@/types/nutrient';

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
