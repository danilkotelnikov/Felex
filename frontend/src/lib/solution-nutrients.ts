// frontend/src/lib/solution-nutrients.ts
import type { DietSolution, NutrientSummary } from '@/types/ration';
import type {
  AlternativeRationSolution,
  NutrientStatus,
  NutrientStatusInfo,
  OptimizationResult,
} from '@/types/optimization';
import type { NormRange } from '@/types/nutrient';

type Species = 'cattle' | 'swine' | 'poultry';

interface NutrientConfig {
  key: keyof NutrientSummary;
  abbrKey: string;
  unit: 'mj' | 'g';
}

const SPECIES_NUTRIENTS: Record<Species, NutrientConfig[]> = {
  cattle: [
    { key: 'energy_oe_cattle', abbrKey: 'energy_oe_cattle', unit: 'mj' },
    { key: 'crude_protein', abbrKey: 'crude_protein', unit: 'g' },
    { key: 'calcium', abbrKey: 'calcium', unit: 'g' },
    { key: 'phosphorus', abbrKey: 'phosphorus', unit: 'g' },
    { key: 'crude_fiber', abbrKey: 'crude_fiber', unit: 'g' },
  ],
  swine: [
    { key: 'energy_oe_pig', abbrKey: 'energy_oe_swine', unit: 'mj' },
    { key: 'lysine', abbrKey: 'lysine', unit: 'g' },
    { key: 'crude_protein', abbrKey: 'crude_protein', unit: 'g' },
    { key: 'phosphorus', abbrKey: 'phosphorus', unit: 'g' },
  ],
  poultry: [
    { key: 'energy_oe_poultry', abbrKey: 'energy_oe_poultry', unit: 'mj' },
    { key: 'methionine_cystine', abbrKey: 'methionine_cystine', unit: 'g' },
    { key: 'crude_protein', abbrKey: 'crude_protein', unit: 'g' },
    { key: 'phosphorus', abbrKey: 'phosphorus', unit: 'g' },
  ],
};

export function getSpeciesFromGroupId(groupId: string): Species {
  if (groupId.startsWith('cattle')) return 'cattle';
  if (groupId.startsWith('swine')) return 'swine';
  if (groupId.startsWith('poultry')) return 'poultry';
  return 'cattle';
}

export function getNutrientStatusForValue(
  value: number,
  norm: NormRange | undefined
): NutrientStatus {
  if (!norm) return 'ok';

  const min = norm.min ?? 0;
  const max = norm.max ?? Infinity;
  const target = norm.target ?? (min + max) / 2;

  if (value >= min && value <= max) {
    return 'ok';
  }

  // Near boundary: within 10% of range
  const range = max - min || target * 0.2;
  const tolerance = range * 0.1;

  if (value >= min - tolerance && value <= max + tolerance) {
    return 'near';
  }

  return 'out';
}

export function getSolutionNutrientProfile(
  nutrients: NutrientSummary,
  groupId: string,
  norms: Record<string, NormRange>,
  t: (key: string) => string
): NutrientStatusInfo[] {
  const species = getSpeciesFromGroupId(groupId);
  const configs = SPECIES_NUTRIENTS[species];

  return configs.map((config) => {
    const value = nutrients[config.key] as number;
    const norm = norms[config.key as string];
    const status = getNutrientStatusForValue(value, norm);

    return {
      key: config.key as string,
      value,
      status,
      abbr: t(`nutrients.abbr.${config.abbrKey}`),
      unit: t(`nutrients.profileUnits.${config.unit}`),
    };
  });
}

/** Wraps a single DietSolution into an OptimizationResult for the alternatives store. */
export function dietSolutionToOptimizationResult(
  solution: DietSolution,
): OptimizationResult {
  const primary = dietSolutionToAlternativeSolution(solution);
  const alternatives = solution.alternatives ?? [];

  return {
    primary,
    alternatives,
    comparison: buildComparison(primary, alternatives),
  };
}

function dietSolutionToAlternativeSolution(
  solution: DietSolution,
): AlternativeRationSolution {
  return {
    id: 'primary',
    label: 'Optimal',
    feeds: solution.items,
    nutrients: solution.nutrient_summary,
    adequacy_score: 100,
    cost: solution.cost_per_day,
    tags: ['requested_mode'],
    optimization_status: solution.optimization_status,
    applied_strategy: 'single',
    warnings: solution.warnings,
  };
}

function buildComparison(
  primary: AlternativeRationSolution,
  alternatives: AlternativeRationSolution[],
): OptimizationResult['comparison'] {
  const solutions = [primary, ...alternatives];
  const costs = solutions.map((solution) => solution.cost);
  const scores = solutions.map((solution) => solution.adequacy_score);
  const feedNameSets = solutions.map(
    (solution) => new Set(solution.feeds.map((feed) => feed.feed_name)),
  );
  const commonFeedNames = Array.from(feedNameSets[0] ?? []).filter((feedName) =>
    feedNameSets.every((set) => set.has(feedName)),
  );
  const allFeedNames = Array.from(
    new Set(solutions.flatMap((solution) => solution.feeds.map((feed) => feed.feed_name))),
  );
  const differentiators = allFeedNames.filter((feedName) => !commonFeedNames.includes(feedName));

  return {
    cost_range: [Math.min(...costs), Math.max(...costs)],
    score_range: [Math.min(...scores), Math.max(...scores)],
    common_feeds: commonFeedNames,
    differentiators,
  };
}

export function formatNutrientValue(value: number, locale: string): string {
  const isRussian = locale.startsWith('ru');
  const formatted = value.toLocaleString(isRussian ? 'ru-RU' : 'en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: value >= 100 ? 0 : 1,
  });
  return formatted;
}
