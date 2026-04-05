import type { NutrientSummary } from '@/types/ration';
import type { NormRange } from '@/types/nutrient';
import { getNutrientStatus, type NutrientStatus } from '@/lib/nutrient-status';
import {
  getManagedNormEntries as getManagedNormEntriesFromRegistry,
  getNutrientDisplayBasis,
  getNutrientLabel,
  getNutrientMeta,
  getNutrientDisplayUnit as getNutrientDisplayUnitFromRegistry,
  isManagedNutrientKey as isManagedNutrientKeyFromRegistry,
  nutrientGroupFamily,
  type NutrientDisplayBasis,
  type NutrientLanguage,
} from '@/lib/nutrient-registry';

function percentOfFeed(totalValue: number, totalFeedKg: number): number | undefined {
  if (totalFeedKg <= 0) {
    return undefined;
  }
  return totalValue / (totalFeedKg * 10);
}

function perKgFeed(totalValue: number, totalFeedKg: number): number | undefined {
  if (totalFeedKg <= 0) {
    return undefined;
  }
  return totalValue / totalFeedKg;
}

function percentOfDm(totalValue: number, totalDmKg: number): number | undefined {
  if (totalDmKg <= 0) {
    return undefined;
  }
  return totalValue / (totalDmKg * 10);
}

export interface ManagedNutrientDisplayRow {
  key: string;
  label: string;
  actual: number;
  totalActual?: number;
  normMin?: number;
  normTarget?: number;
  normMax?: number;
  unit: string;
  status: NutrientStatus;
  targetPercent?: number;
}

export function isManagedNutrientKey(groupId: string, key: string): boolean {
  return isManagedNutrientKeyFromRegistry(groupId, key);
}

export function getManagedNormEntries(
  groupId: string,
  norms: Record<string, NormRange>,
): Array<[string, NormRange]> {
  return getManagedNormEntriesFromRegistry(groupId, norms);
}

export function nutrientDisplayBasis(groupId: string, key: string): NutrientDisplayBasis {
  return getNutrientDisplayBasis(groupId, key);
}

export function nutrientDisplayUnit(
  groupId: string,
  key: string,
  language: NutrientLanguage,
  fallbackUnit?: string,
): string {
  return getNutrientDisplayUnitFromRegistry(groupId, key, language, fallbackUnit);
}

export function shouldShowAggregateTotal(groupId: string, key: string): boolean {
  const meta = getNutrientMeta(key);
  return nutrientDisplayBasis(groupId, key) === 'absolute' && meta?.category !== 'ratio';
}

export function getNutrientTargetPercent(actual: number, target?: number): number | undefined {
  if (target === undefined || target <= 0) {
    return undefined;
  }
  return (actual / target) * 100;
}

export function getNutrientDisplayActual(
  nutrients: NutrientSummary,
  groupId: string,
  key: string,
): number | undefined {
  const family = nutrientGroupFamily(groupId);

  switch (key) {
    case 'dry_matter_intake':
      return nutrients.total_dm_kg;
    case 'feed_intake':
      return nutrients.total_weight_kg;
    case 'starch_pct_dm':
      return nutrients.starch_pct_dm;
    case 'energy_oe_poultry':
      if (family === 'poultry') {
        return perKgFeed(nutrients.energy_oe_poultry, nutrients.total_weight_kg);
      }
      return nutrients.energy_oe_poultry;
    case 'crude_protein':
      if (family === 'swine') {
        return perKgFeed(nutrients.crude_protein, nutrients.total_weight_kg);
      }
      if (family === 'poultry') {
        return percentOfFeed(nutrients.crude_protein, nutrients.total_weight_kg);
      }
      return nutrients.crude_protein;
    case 'crude_protein_pct':
      return percentOfFeed(nutrients.crude_protein, nutrients.total_weight_kg);
    case 'cp_pct_dm':
      return nutrients.cp_pct_dm;
    case 'dig_protein_cattle_pct_cp':
      return nutrients.dig_protein_cattle_pct_cp;
    case 'lysine':
      if (family === 'swine') {
        return perKgFeed(nutrients.lysine, nutrients.total_weight_kg);
      }
      if (family === 'poultry') {
        return percentOfFeed(nutrients.lysine, nutrients.total_weight_kg);
      }
      return nutrients.lysine;
    case 'lysine_sid':
      if (family === 'swine' && !groupId.includes('sow')) {
        return perKgFeed(nutrients.lysine, nutrients.total_weight_kg);
      }
      return nutrients.lysine;
    case 'lysine_sid_pct':
    case 'lysine_tid_pct':
      return percentOfFeed(nutrients.lysine, nutrients.total_weight_kg);
    case 'methionine_cystine':
      if (family === 'swine') {
        return perKgFeed(nutrients.methionine_cystine, nutrients.total_weight_kg);
      }
      if (family === 'poultry') {
        return percentOfFeed(nutrients.methionine_cystine, nutrients.total_weight_kg);
      }
      return nutrients.methionine_cystine;
    case 'methionine_cystine_sid':
      if (family === 'swine' && !groupId.includes('sow')) {
        return perKgFeed(nutrients.methionine_cystine, nutrients.total_weight_kg);
      }
      return nutrients.methionine_cystine;
    case 'methionine_cystine_tid_pct':
      return percentOfFeed(nutrients.methionine_cystine, nutrients.total_weight_kg);
    case 'calcium':
      if (family === 'swine') {
        return perKgFeed(nutrients.calcium, nutrients.total_weight_kg);
      }
      if (family === 'poultry') {
        return percentOfFeed(nutrients.calcium, nutrients.total_weight_kg);
      }
      return nutrients.calcium;
    case 'calcium_pct':
      return percentOfFeed(nutrients.calcium, nutrients.total_weight_kg);
    case 'ca_p_ratio':
      return nutrients.ca_p_ratio;
    case 'methionine_cystine_lys_ratio':
      return nutrients.lysine > 0 ? nutrients.methionine_cystine / nutrients.lysine : undefined;
    default: {
      const value = (nutrients as unknown as Record<string, number | undefined>)[key];
      if (!Number.isFinite(value)) {
        return undefined;
      }
      const numericValue = value as number;
      switch (nutrientDisplayBasis(groupId, key)) {
        case 'per_kg_feed':
          return perKgFeed(numericValue, nutrients.total_weight_kg);
        case 'percent_of_feed':
          return percentOfFeed(numericValue, nutrients.total_weight_kg);
        case 'percent_of_dm':
          return percentOfDm(numericValue, nutrients.total_dm_kg);
        default:
          return numericValue;
      }
    }
  }
}

export function buildManagedNutrientRows(
  nutrients: NutrientSummary | null | undefined,
  groupId: string,
  norms: Record<string, NormRange>,
  language: NutrientLanguage,
  animalCount = 1,
): ManagedNutrientDisplayRow[] {
  if (!nutrients) {
    return [];
  }

  return getManagedNormEntries(groupId, norms)
    .flatMap(([key, norm]) => {
      const rawActual = getNutrientDisplayActual(nutrients, groupId, key);
      if (rawActual === undefined) {
        return [];
      }

      const actual = rawActual;
      return [{
        key,
        label: getNutrientLabel(key, language),
        actual,
        totalActual: shouldShowAggregateTotal(groupId, key)
          ? actual * animalCount
          : undefined,
        normMin: norm.min,
        normTarget: norm.target,
        normMax: norm.max,
        unit: nutrientDisplayUnit(groupId, key, language),
        status: getNutrientStatus(actual, norm.min, norm.max, norm.target),
        targetPercent: getNutrientTargetPercent(actual, norm.target),
      }];
    })
    .filter((row) =>
      row.normMin !== undefined || row.normTarget !== undefined || row.normMax !== undefined,
    );
}
