import type { AnimalProperties } from '@/stores/rationStore';
import {
  getBaseNormsForGroupId as getFallbackBaseNormsForGroupId,
  getPresetsForGroup,
  type NormPreset,
  type NormRange,
} from '@/types/nutrient';
import { resolveAnimalGroupId } from '@/stores/rationStore';
import { getNutrientMeta } from '@/lib/nutrient-registry';

interface PresetReference {
  norms: Record<string, NormRange>;
  params: {
    weight?: number;
    milkYield?: number;
    fatPct?: number;
    dailyGain?: number;
    eggProduction?: number;
    age?: number;
  };
  label?: string;
}

function findPresetById(groupId: string, presetId: string): NormPreset | undefined {
  return getPresetsForGroup(groupId).find((preset) => preset.id === presetId);
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function lerp(a: number | undefined, b: number | undefined, t: number): number | undefined {
  if (a === undefined && b === undefined) return undefined;
  if (a === undefined) return b;
  if (b === undefined) return a;
  return a + (b - a) * t;
}

function roundValue(value: number | undefined): number | undefined {
  if (value === undefined) return undefined;
  if (Math.abs(value) >= 100) return Math.round(value);
  if (Math.abs(value) >= 10) return Math.round(value * 10) / 10;
  return Math.round(value * 100) / 100;
}

function mergeRanges(a: NormRange | undefined, b: NormRange | undefined, t: number): NormRange {
  return {
    min: roundValue(lerp(a?.min, b?.min, t)),
    target: roundValue(lerp(a?.target, b?.target, t)),
    max: roundValue(lerp(a?.max, b?.max, t)),
  };
}

function mergeNormSets(a: Record<string, NormRange>, b: Record<string, NormRange>, t: number): Record<string, NormRange> {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  const merged: Record<string, NormRange> = {};

  for (const key of keys) {
    merged[key] = mergeRanges(a[key], b[key], t);
  }

  return merged;
}

function buildReference(a: PresetReference, b: PresetReference, t: number): PresetReference {
  return {
    norms: mergeNormSets(a.norms, b.norms, t),
    params: {
      weight: lerp(a.params.weight, b.params.weight, t),
      milkYield: lerp(a.params.milkYield, b.params.milkYield, t),
      fatPct: lerp(a.params.fatPct, b.params.fatPct, t),
      dailyGain: lerp(a.params.dailyGain, b.params.dailyGain, t),
      eggProduction: lerp(a.params.eggProduction, b.params.eggProduction, t),
      age: lerp(a.params.age, b.params.age, t),
    },
    label: a.label ?? b.label,
  };
}

function parseAgeHint(preset: NormPreset): number | undefined {
  const age = preset.params.age;
  if (!age) return undefined;
  const matches = age.match(/(\d+)/g);
  if (!matches || matches.length === 0) return undefined;
  const values = matches.map((value) => Number.parseInt(value, 10)).filter((value) => Number.isFinite(value));
  if (values.length === 0) return undefined;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function presetToReference(preset: NormPreset): PresetReference {
  return {
    norms: preset.norms,
    params: {
      weight: preset.params.weight,
      milkYield: preset.params.milkYield,
      fatPct: preset.params.fatPct,
      dailyGain: preset.params.dailyGain,
      eggProduction: preset.params.eggProduction,
      age: parseAgeHint(preset),
    },
    label: preset.label_ru,
  };
}

function resolveMetric(groupId: string, properties: AnimalProperties): { key: keyof PresetReference['params']; value: number | undefined } {
  if (groupId === 'cattle_dairy') {
    return { key: 'milkYield', value: properties.milkYieldKg };
  }
  if (groupId === 'cattle_beef') {
    return { key: 'weight', value: properties.liveWeightKg };
  }
  if (groupId === 'swine_finisher') {
    return { key: 'weight', value: properties.liveWeightKg };
  }
  if (groupId === 'poultry_broiler') {
    return { key: 'age', value: properties.ageToDays ?? properties.ageFromDays };
  }
  if (groupId === 'poultry_layer') {
    return {
      key: 'eggProduction',
      value: properties.eggProductionPerYear ?? (properties.ageToDays && properties.ageToDays > 315 ? 285 : 320),
    };
  }
  return { key: 'weight', value: properties.liveWeightKg };
}

function getBreedAdjustment(groupId: string, breed: string): { energy: number; protein: number; mineral: number; vitamin: number } {
  const normalized = breed.toLowerCase();

  if (groupId === 'cattle_dairy') {
    if (normalized.includes('голш')) return { energy: 0.05, protein: 0.05, mineral: 0.03, vitamin: 0.03 };
    if (normalized.includes('джерс')) return { energy: -0.02, protein: 0.01, mineral: 0.05, vitamin: 0.02 };
    if (normalized.includes('айрш')) return { energy: 0.01, protein: 0.02, mineral: 0.02, vitamin: 0.02 };
    if (normalized.includes('симмент')) return { energy: 0.02, protein: 0.02, mineral: 0.02, vitamin: 0.01 };
    return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
  }

  if (groupId === 'cattle_beef') {
    if (normalized.includes('ангус')) return { energy: 0.03, protein: 0.03, mineral: 0.01, vitamin: 0.01 };
    if (normalized.includes('шароле')) return { energy: 0.04, protein: 0.04, mineral: 0.02, vitamin: 0.02 };
    if (normalized.includes('лимуз')) return { energy: 0.03, protein: 0.03, mineral: 0.02, vitamin: 0.01 };
    return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
  }

  if (groupId.startsWith('swine')) {
    if (normalized.includes('дюрок')) return { energy: 0.03, protein: 0.03, mineral: 0.01, vitamin: 0.01 };
    if (normalized.includes('ландрас')) return { energy: 0.02, protein: 0.03, mineral: 0.01, vitamin: 0.01 };
    if (normalized.includes('пьетрен')) return { energy: 0.01, protein: 0.04, mineral: 0.01, vitamin: 0.01 };
    return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
  }

  if (groupId === 'poultry_broiler') {
    if (normalized.includes('кобб')) return { energy: 0.02, protein: 0.03, mineral: 0.01, vitamin: 0.01 };
    if (normalized.includes('росс')) return { energy: 0.03, protein: 0.03, mineral: 0.01, vitamin: 0.01 };
  }

  if (groupId === 'poultry_layer') {
    if (normalized.includes('ломанн')) return { energy: 0.01, protein: 0.02, mineral: 0.03, vitamin: 0.02 };
    if (normalized.includes('хайсекс')) return { energy: 0.01, protein: 0.02, mineral: 0.03, vitamin: 0.02 };
  }

  return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
}

function getSexAdjustment(groupId: string, sex: AnimalProperties['sex']) {
  if (groupId === 'cattle_dairy' || groupId === 'poultry_layer' || groupId === 'swine_sow') {
    return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
  }

  if (sex === 'male') {
    return { energy: 0.03, protein: 0.03, mineral: 0.01, vitamin: 0.0 };
  }
  if (sex === 'female') {
    return { energy: -0.01, protein: -0.01, mineral: 0.0, vitamin: 0.0 };
  }

  return { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };
}

function getSpecialStageReference(groupId: string, properties: AnimalProperties): PresetReference | null {
  if (groupId === 'swine_sow') {
    const stage = properties.reproductiveStage ?? ((properties.litterSize ?? 0) > 0 ? 'lactation' : 'gestation');
    const presetId = stage === 'lactation' ? 'swine_sow_lactation' : 'swine_sow_gestation';
    const preset = findPresetById(groupId, presetId);
    return preset ? presetToReference(preset) : null;
  }

  return null;
}

function interpolatePresets(groupId: string, properties: AnimalProperties): PresetReference {
  const stageSpecific = getSpecialStageReference(groupId, properties);
  if (stageSpecific) {
    return stageSpecific;
  }

  const presets = getPresetsForGroup(groupId).map(presetToReference);
  if (presets.length === 0) {
    return {
      norms: getFallbackBaseNormsForGroupId(groupId),
      params: {},
    };
  }

  const metric = resolveMetric(groupId, properties);
  const withMetric = presets.filter((preset) => preset.params[metric.key] !== undefined);
  if (metric.value === undefined || withMetric.length === 0) {
    return presets[presets.length - 1];
  }

  const sorted = [...withMetric].sort((left, right) => (left.params[metric.key] ?? 0) - (right.params[metric.key] ?? 0));

  if (metric.value <= (sorted[0].params[metric.key] ?? 0)) {
    return sorted[0];
  }
  const last = sorted[sorted.length - 1];
  if (metric.value >= (last.params[metric.key] ?? 0)) {
    return last;
  }

  for (let index = 0; index < sorted.length - 1; index += 1) {
    const current = sorted[index];
    const next = sorted[index + 1];
    const currentValue = current.params[metric.key] ?? 0;
    const nextValue = next.params[metric.key] ?? 0;

    if (metric.value >= currentValue && metric.value <= nextValue) {
      const range = nextValue - currentValue || 1;
      const t = clamp((metric.value - currentValue) / range, 0, 1);
      return buildReference(current, next, t);
    }
  }

  return last;
}

function scaleNormValue(value: number | undefined, factor: number): number | undefined {
  if (value === undefined) return undefined;
  return roundValue(value * factor);
}

function applyAdjustments(groupId: string, properties: AnimalProperties, reference: PresetReference): Record<string, NormRange> {
  const breed = getBreedAdjustment(groupId, properties.breed);
  const sex = getSexAdjustment(groupId, properties.sex);
  const referenceWeight = reference.params.weight ?? properties.liveWeightKg;
  const currentWeight = properties.liveWeightKg || referenceWeight || 1;
  const weightDeltaPct = referenceWeight ? (currentWeight - referenceWeight) / referenceWeight : 0;
  const gainRef = reference.params.dailyGain ?? properties.dailyGainG;
  const gainDeltaPct = gainRef && properties.dailyGainG ? (properties.dailyGainG - gainRef) / gainRef : 0;
  const milkRef = reference.params.milkYield ?? properties.milkYieldKg;
  const milkDelta = milkRef && properties.milkYieldKg ? properties.milkYieldKg - milkRef : 0;
  const fatRef = reference.params.fatPct ?? properties.milkFatPct ?? 3.7;
  const fatDelta = (properties.milkFatPct ?? fatRef) - fatRef;
  const eggRef = reference.params.eggProduction ?? properties.eggProductionPerYear;
  const eggDeltaPct = eggRef && properties.eggProductionPerYear ? (properties.eggProductionPerYear - eggRef) / eggRef : 0;
  const stageBoost = groupId === 'swine_sow' && (properties.reproductiveStage ?? 'gestation') === 'lactation'
    ? { energy: 0.18, protein: 0.22, mineral: 0.12, vitamin: 0.08 }
    : { energy: 0.0, protein: 0.0, mineral: 0.0, vitamin: 0.0 };

  const energyFactor = clamp(
    1
      + weightDeltaPct * (groupId === 'cattle_dairy' ? 0.18 : 0.35)
      + gainDeltaPct * 0.45
      + milkDelta * 0.018
      + fatDelta * 0.03
      + eggDeltaPct * 0.18
      + breed.energy
      + sex.energy
      + stageBoost.energy,
    0.8,
    1.35,
  );

  const proteinFactor = clamp(
    1
      + weightDeltaPct * (groupId === 'cattle_dairy' ? 0.12 : 0.24)
      + gainDeltaPct * 0.55
      + milkDelta * 0.022
      + fatDelta * 0.025
      + eggDeltaPct * 0.22
      + breed.protein
      + sex.protein
      + stageBoost.protein,
    0.82,
    1.45,
  );

  const mineralFactor = clamp(
    1
      + weightDeltaPct * 0.16
      + milkDelta * 0.012
      + fatDelta * 0.04
      + eggDeltaPct * 0.28
      + breed.mineral
      + sex.mineral
      + stageBoost.mineral,
    0.85,
    1.45,
  );

  const vitaminFactor = clamp(
    1
      + weightDeltaPct * 0.08
      + milkDelta * 0.01
      + eggDeltaPct * 0.12
      + breed.vitamin
      + sex.vitamin
      + stageBoost.vitamin,
    0.85,
    1.3,
  );

  const fiberFactor = clamp(
    1 + weightDeltaPct * 0.05 - milkDelta * 0.004 + eggDeltaPct * 0.04,
    0.9,
    1.12,
  );

  const adjusted: Record<string, NormRange> = {};

  for (const [key, range] of Object.entries(reference.norms)) {
    const category = getNutrientMeta(key)?.category ?? 'mineral';
    const factor = category === 'energy'
      ? energyFactor
      : category === 'protein'
        ? proteinFactor
        : category === 'vitamin'
          ? vitaminFactor
          : category === 'fiber'
            ? fiberFactor
            : mineralFactor;

    adjusted[key] = {
      min: scaleNormValue(range.min, factor),
      target: scaleNormValue(range.target, factor),
      max: scaleNormValue(range.max, factor),
    };
  }

  return adjusted;
}

export function getNormReferenceLabel(properties: AnimalProperties): string | null {
  return getNormReferenceLabelForPreset(properties);
}

export function getNormReferenceLabelForPreset(
  properties: AnimalProperties,
  presetId?: string | null,
): string | null {
  const groupId = resolveAnimalGroupId(properties);
  const selectedPreset = presetId
    ? findPresetById(groupId, presetId)
    : null;
  if (selectedPreset) {
    return selectedPreset.label_ru;
  }

  const reference = interpolatePresets(groupId, properties);
  return reference.label ?? null;
}

export function getNormsForAnimal(
  properties: AnimalProperties,
  presetId?: string | null,
): Record<string, NormRange> {
  const groupId = resolveAnimalGroupId(properties);
  const selectedPreset = presetId
    ? findPresetById(groupId, presetId)
    : null;
  if (selectedPreset) {
    return selectedPreset.norms;
  }

  const reference = interpolatePresets(groupId, properties);
  return applyAdjustments(groupId, properties, reference);
}

export function getBaseNormsForGroupId(groupId: string): Record<string, NormRange> {
  return getFallbackBaseNormsForGroupId(groupId);
}
