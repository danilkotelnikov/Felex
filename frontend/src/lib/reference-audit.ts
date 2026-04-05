import type { NormRange } from '@/types/nutrient';
import type { NutrientSummary } from '@/types/ration';
import type { AnimalProperties } from '@/stores/rationStore';
import { getManagedNormEntries, getNutrientDisplayActual } from '@/lib/nutrient-display';
import { getNutrientLabels, getNutrientUnit } from '@/lib/nutrient-registry';
import { getNutrientStatus } from '@/lib/nutrient-status';
import { RATION_PRESETS } from '@/data/ration-presets';

type ReferenceDriftDirection = 'low' | 'high';
type GroupFamily = 'cattle' | 'swine' | 'poultry';
type VitaminKey =
  | 'carotene'
  | 'vit_d3'
  | 'vit_e';

export interface ReferenceDriftItem {
  key: string;
  name_ru: string;
  name_en: string;
  actual: number;
  direction: ReferenceDriftDirection;
}

export interface AdvisoryVitaminItem {
  key: VitaminKey;
  name_ru: string;
  name_en: string;
  actual: number;
  unit: string;
}

export interface PresetBaselineAudit {
  preset_id: string;
  preset_name_ru: string;
  preset_name_en: string;
  status: 'aligned' | 'drifted';
  drift_items: ReferenceDriftItem[];
}

const MATERIAL_KEYS_BY_FAMILY: Record<GroupFamily, Set<string>> = {
  cattle: new Set(['energy_eke', 'crude_protein', 'crude_fiber', 'calcium', 'phosphorus']),
  swine: new Set([
    'energy_oe_pig',
    'crude_protein',
    'lysine',
    'methionine_cystine',
    'calcium',
    'phosphorus',
  ]),
  poultry: new Set([
    'energy_oe_poultry',
    'crude_protein',
    'methionine_cystine',
    'calcium',
    'phosphorus',
  ]),
};

const RATION_PRESET_BY_NORM_PRESET: Record<string, string> = {
  cattle_dairy_25: 'dairy_25',
  cattle_dairy_35: 'dairy_35',
  cattle_beef_400: 'beef_400',
  cattle_beef_500: 'beef_550',
  swine_starter: 'swine_starter',
  swine_finisher_preset: 'swine_finisher',
  swine_sow_gestation: 'swine_sow_gestation',
  swine_sow_lactation: 'swine_sow_lactation',
  poultry_broiler_starter: 'broiler_starter',
  poultry_broiler_finisher: 'broiler_finisher',
  poultry_layer_phase1: 'layer_phase1',
  poultry_layer_phase2: 'layer_phase2',
};

function groupFamily(groupId: string): GroupFamily {
  if (groupId.startsWith('swine')) {
    return 'swine';
  }
  if (groupId.startsWith('poultry')) {
    return 'poultry';
  }
  return 'cattle';
}

function normalizeFeedName(value: string | undefined): string {
  return (value ?? '').trim().toLowerCase();
}

function isHighYieldDairyContext(
  groupId: string,
  animalProperties?: AnimalProperties,
  activeNormPresetId?: string | null,
): boolean {
  if (!groupId.startsWith('cattle_dairy')) {
    return false;
  }
  if (activeNormPresetId === 'cattle_dairy_35') {
    return true;
  }
  return (animalProperties?.milkYieldKg ?? 0) >= 30;
}

function advisoryVitaminKeys(
  groupId: string,
  animalProperties?: AnimalProperties,
  activeNormPresetId?: string | null,
): VitaminKey[] {
  if (groupId.startsWith('cattle')) {
    return isHighYieldDairyContext(groupId, animalProperties, activeNormPresetId)
      ? ['carotene', 'vit_d3', 'vit_e']
      : ['carotene', 'vit_d3', 'vit_e'];
  }

  if (groupId.startsWith('swine')) {
    return ['vit_d3', 'vit_e'];
  }

  if (groupId.startsWith('poultry_layer')) {
    return ['vit_d3', 'vit_e'];
  }

  return ['vit_d3', 'vit_e'];
}

function driftDirection(actual: number, norm: NormRange): ReferenceDriftDirection {
  if (norm.max !== undefined && actual > norm.max) {
    return 'high';
  }

  const reference = norm.target ?? norm.min;
  if (reference !== undefined && actual >= reference) {
    return 'high';
  }

  return 'low';
}

export function getReferenceDriftItems(
  groupId: string,
  nutrients: NutrientSummary,
  norms: Record<string, NormRange>,
): ReferenceDriftItem[] {
  const family = groupFamily(groupId);
  const materialKeys = MATERIAL_KEYS_BY_FAMILY[family];

  return getManagedNormEntries(groupId, norms).flatMap(([key, norm]) => {
    if (!materialKeys.has(key)) {
      return [];
    }

    const actual = getNutrientDisplayActual(nutrients, groupId, key);
    if (actual === undefined) {
      return [];
    }

    const status = getNutrientStatus(actual, norm.min, norm.max, norm.target);
    if (status === 'ok') {
      return [];
    }

    const labels = getNutrientLabels(key);
    if (!labels) {
      return [];
    }

    return {
      key,
      name_ru: labels.name_ru,
      name_en: labels.name_en,
      actual,
      direction: driftDirection(actual, norm),
    };
  });
}

export function getVitaminPolicyKey(
  groupId: string,
  animalProperties?: AnimalProperties,
  activeNormPresetId?: string | null,
): string {
  if (groupId.startsWith('cattle')) {
    return isHighYieldDairyContext(groupId, animalProperties, activeNormPresetId)
      ? 'nutrients.vitaminPolicyCattleDairyHighYield'
      : 'nutrients.vitaminPolicyCattleGeneral';
  }
  if (groupId.startsWith('swine')) {
    return animalProperties?.reproductiveStage === 'lactation' || groupId.includes('sow')
      ? 'nutrients.vitaminPolicySwineBreeding'
      : 'nutrients.vitaminPolicySwineFattening';
  }
  return groupId.startsWith('poultry_layer')
    ? 'nutrients.vitaminPolicyPoultryLayer'
    : 'nutrients.vitaminPolicyPoultryBroiler';
}

export function getAdvisoryVitaminItems(
  groupId: string,
  nutrients: NutrientSummary,
  animalProperties?: AnimalProperties,
  activeNormPresetId?: string | null,
): AdvisoryVitaminItem[] {
  const advisoryKeys = advisoryVitaminKeys(groupId, animalProperties, activeNormPresetId);

  return advisoryKeys.flatMap((key) => {
    const labels = getNutrientLabels(key);
    const actual = nutrients[key] as number | undefined;
    if (!labels || actual === undefined || actual <= 0) {
      return [];
    }

    return {
      key,
      name_ru: labels.name_ru,
      name_en: labels.name_en,
      actual,
      unit: getNutrientUnit(key, 'ru'),
    };
  });
}

export function getPresetBaselineAudit(
  items: Array<{ feed: { name_ru: string; name_en?: string }; amount_kg: number }>,
  groupId: string,
  nutrients: NutrientSummary,
  norms: Record<string, NormRange>,
  activeNormPresetId?: string | null,
): PresetBaselineAudit | null {
  const rationPresetId = activeNormPresetId
    ? RATION_PRESET_BY_NORM_PRESET[activeNormPresetId]
    : undefined;
  if (!rationPresetId) {
    return null;
  }

  const preset = RATION_PRESETS.find((candidate) => candidate.id === rationPresetId);
  if (!preset || preset.items.length !== items.length) {
    return null;
  }

  const currentByName = new Map(
    items.flatMap((item) => {
      const pairs: Array<[string, number]> = [[normalizeFeedName(item.feed.name_ru), item.amount_kg]];
      if (item.feed.name_en) {
        pairs.push([normalizeFeedName(item.feed.name_en), item.amount_kg]);
      }
      return pairs;
    }),
  );

  const matchesPreset = preset.items.every((item) => {
    const currentAmount = currentByName.get(normalizeFeedName(item.feedName));
    if (currentAmount === undefined) {
      return false;
    }

    const tolerance = Math.max(0.01, item.kgPerDay * 0.03);
    return Math.abs(currentAmount - item.kgPerDay) <= tolerance;
  });

  if (!matchesPreset) {
    return null;
  }

  const driftItems = getReferenceDriftItems(groupId, nutrients, norms);

  return {
    preset_id: preset.id,
    preset_name_ru: preset.name_ru,
    preset_name_en: preset.name_en,
    status: driftItems.length > 0 ? 'drifted' : 'aligned',
    drift_items: driftItems,
  };
}
