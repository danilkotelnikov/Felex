import { useEffect, useMemo, useState } from 'react';
import { animalsApi } from '@/lib/api';
import { getNormReferenceLabelForPreset, getNormsForAnimal } from '@/lib/norms';
import { resolveAnimalGroupId, type AnimalProperties } from '@/stores/rationStore';
import type { OptimizeAnimalPropertiesPayload } from '@/types/ration';
import type { AnimalNorm, NormMethodology, NormRange } from '@/types/nutrient';

interface ResolvedNormApiValue {
  resolved_group_id: string;
  norm: AnimalNorm;
  methodology?: NormMethodology | null;
}

interface UseResolvedNormReferenceResult {
  norms: Record<string, NormRange>;
  baseNorms: Record<string, NormRange>;
  referenceLabel: string | null;
  resolvedGroupId: string;
  normSource: 'backend' | 'local';
  methodology: NormMethodology | null;
}

const resolvedNormCache = new Map<string, ResolvedNormApiValue>();
const inflightNormRequests = new Map<string, Promise<ResolvedNormApiValue | null>>();

function buildAnimalPayload(properties: AnimalProperties): OptimizeAnimalPropertiesPayload {
  return {
    species: properties.species,
    production_type: properties.productionType,
    breed: properties.breed,
    sex: properties.sex,
    live_weight_kg: properties.liveWeightKg,
    age_from_days: properties.ageFromDays,
    age_to_days: properties.ageToDays,
    milk_yield_kg: properties.milkYieldKg,
    milk_fat_pct: properties.milkFatPct,
    daily_gain_g: properties.dailyGainG,
    egg_production_per_year: properties.eggProductionPerYear,
    litter_size: properties.litterSize,
    reproductive_stage: properties.reproductiveStage,
  };
}

function buildCacheKey(groupId: string, properties: AnimalProperties, presetId?: string | null): string {
  return JSON.stringify({
    groupId,
    presetId: presetId ?? null,
    animal_properties: buildAnimalPayload(properties),
  });
}

function animalNormToRanges(norm: AnimalNorm): Record<string, NormRange> {
  const keys = new Set([
    ...Object.keys(norm.nutrients_min ?? {}),
    ...Object.keys(norm.nutrients_target ?? {}),
    ...Object.keys(norm.nutrients_max ?? {}),
  ]);
  const ranges: Record<string, NormRange> = {};

  for (const key of keys) {
    ranges[key] = {
      min: norm.nutrients_min?.[key],
      target: norm.nutrients_target?.[key],
      max: norm.nutrients_max?.[key],
    };
  }

  const intakeKey = norm.species === 'cattle' ? 'dry_matter_intake' : 'feed_intake';
  if (norm.feed_intake_min !== undefined || norm.feed_intake_max !== undefined) {
    ranges[intakeKey] = {
      min: norm.feed_intake_min,
      target:
        norm.feed_intake_min !== undefined && norm.feed_intake_max !== undefined
          ? Math.round((((norm.feed_intake_min + norm.feed_intake_max) / 2) * 100)) / 100
          : undefined,
      max: norm.feed_intake_max,
    };
  }

  return ranges;
}

async function fetchResolvedNorm(
  groupId: string,
  properties: AnimalProperties,
  presetId?: string | null,
): Promise<ResolvedNormApiValue | null> {
  try {
    const response = await animalsApi.resolveNorms(groupId, {
      norm_preset_id: presetId ?? null,
      animal_properties: buildAnimalPayload(properties),
    });
    return response.data;
  } catch {
    return null;
  }
}

export function useResolvedNormReference(
  animalProperties: AnimalProperties,
  presetId?: string | null,
  customNorms?: Record<string, NormRange> | null,
): UseResolvedNormReferenceResult {
  const fallbackGroupId = resolveAnimalGroupId(animalProperties);
  const fallbackNorms = useMemo(
    () => getNormsForAnimal(animalProperties, presetId),
    [animalProperties, presetId],
  );
  const referenceLabel = useMemo(
    () => getNormReferenceLabelForPreset(animalProperties, presetId),
    [animalProperties, presetId],
  );
  const cacheKey = useMemo(
    () => buildCacheKey(fallbackGroupId, animalProperties, presetId),
    [animalProperties, fallbackGroupId, presetId],
  );
  const [resolved, setResolved] = useState<ResolvedNormApiValue | null>(() =>
    resolvedNormCache.get(cacheKey) ?? null,
  );

  useEffect(() => {
    let cancelled = false;
    const cached = resolvedNormCache.get(cacheKey);
    if (cached) {
      setResolved(cached);
      return () => {
        cancelled = true;
      };
    }

    const existing = inflightNormRequests.get(cacheKey);
    const request =
      existing ??
      fetchResolvedNorm(fallbackGroupId, animalProperties, presetId).then((result) => {
        if (result) {
          resolvedNormCache.set(cacheKey, result);
        }
        inflightNormRequests.delete(cacheKey);
        return result;
      });

    if (!existing) {
      inflightNormRequests.set(cacheKey, request);
    }

    void request.then((result) => {
      if (!cancelled) {
        setResolved(result);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [animalProperties, cacheKey, fallbackGroupId, presetId]);

  const baseNorms = resolved ? animalNormToRanges(resolved.norm) : fallbackNorms;

  return {
    norms: {
      ...baseNorms,
      ...(customNorms ?? {}),
    },
    baseNorms,
    referenceLabel,
    resolvedGroupId: resolved?.resolved_group_id ?? fallbackGroupId,
    normSource: resolved ? 'backend' : 'local',
    methodology: resolved?.methodology ?? null,
  };
}
