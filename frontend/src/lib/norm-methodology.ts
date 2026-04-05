import type { TFunction } from 'i18next';
import type { NormMethodology, NormMethodologyFactor, NormMethodologyMetric } from '@/types/nutrient';

interface MethodologyCopy {
  title: string;
  description: string;
  formulaMarkdown: string;
}

interface CitationEntry {
  gost: string;
  harvard: string;
}

const METHODOLOGY_COPY_KEYS: Record<string, {
  title: string;
  description: string;
  formulas: string;
}> = {
  reference_table: {
    title: 'norms.methodologyMode.referenceTableTitle',
    description: 'norms.methodologyMode.referenceTableDescription',
    formulas: 'norms.methodologyMode.referenceTableFormulas',
  },
  dairy_lactation_interpolation: {
    title: 'norms.methodologyMode.dairyInterpolationTitle',
    description: 'norms.methodologyMode.dairyInterpolationDescription',
    formulas: 'norms.methodologyMode.dairyInterpolationFormulas',
  },
  swine_finisher_interpolation: {
    title: 'norms.methodologyMode.swineInterpolationTitle',
    description: 'norms.methodologyMode.swineInterpolationDescription',
    formulas: 'norms.methodologyMode.swineInterpolationFormulas',
  },
  context_scaling: {
    title: 'norms.methodologyMode.contextScalingTitle',
    description: 'norms.methodologyMode.contextScalingDescription',
    formulas: 'norms.methodologyMode.contextScalingFormulas',
  },
};

const METRIC_LABEL_KEYS: Record<string, string> = {
  live_weight_kg: 'norms.methodologyMetric.liveWeight',
  milk_yield_kg: 'norms.methodologyMetric.milkYield',
  milk_fat_pct: 'norms.methodologyMetric.milkFat',
  fat_corrected_milk_kg: 'norms.methodologyMetric.fatCorrectedMilk',
  dry_matter_intake_modeled: 'norms.methodologyMetric.modeledDryMatterIntake',
  dry_matter_intake_min: 'norms.methodologyMetric.dryMatterIntakeMin',
  dry_matter_intake_max: 'norms.methodologyMetric.dryMatterIntakeMax',
  daily_gain_g: 'norms.methodologyMetric.dailyGain',
  feed_intake_modeled: 'norms.methodologyMetric.modeledFeedIntake',
  energy_oe_pig_modeled: 'norms.methodologyMetric.modeledSwineEnergy',
  crude_protein_modeled: 'norms.methodologyMetric.modeledCrudeProtein',
  lysine_sid_modeled: 'norms.methodologyMetric.modeledSidLysine',
  methionine_cystine_sid_modeled: 'norms.methodologyMetric.modeledSidMethionineCystine',
  egg_production_per_year: 'norms.methodologyMetric.eggProduction',
  age_days: 'norms.methodologyMetric.ageDays',
  feed_intake_min: 'norms.methodologyMetric.feedIntakeMin',
  feed_intake_max: 'norms.methodologyMetric.feedIntakeMax',
};

const FACTOR_LABEL_KEYS: Record<string, string> = {
  intake_factor: 'norms.methodologyFactor.intake',
  energy_factor: 'norms.methodologyFactor.energy',
  protein_factor: 'norms.methodologyFactor.protein',
  mineral_factor: 'norms.methodologyFactor.mineral',
  vitamin_factor: 'norms.methodologyFactor.vitamin',
  fiber_factor: 'norms.methodologyFactor.fiber',
  amino_factor: 'norms.methodologyFactor.amino',
};

const UNIT_LABEL_KEYS: Record<string, string> = {
  kg: 'units.kg',
  kg_day: 'animal.kgDay',
  pct: 'units.percent',
  g_day: 'norms.gDay',
  eggs_year: 'animal.eggsPerYear',
  days: 'norms.methodologyUnit.days',
  mj_day: 'norms.methodologyUnit.mjDay',
  g_kg_feed: 'norms.methodologyUnit.gKgFeed',
};

const CATALOG_URLS = {
  dairy: 'https://nap.nationalacademies.org/catalog/25806/nutrient-requirements-of-dairy-cattle-eighth-revised-edition',
  beef: 'https://nap.nationalacademies.org/catalog/19014/nutrient-requirements-of-beef-cattle-eighth-revised-edition',
  swine: 'https://nap.nationalacademies.org/catalog/13298/nutrient-requirements-of-swine-eleventh-revised-edition',
  poultry: 'https://nap.nationalacademies.org/catalog/2114/nutrient-requirements-of-poultry-ninth-revised-edition-1994',
};

function methodologyCopyKey(methodology: NormMethodology | null | undefined) {
  if (!methodology) {
    return METHODOLOGY_COPY_KEYS.reference_table;
  }
  return METHODOLOGY_COPY_KEYS[methodology.key] ?? METHODOLOGY_COPY_KEYS.context_scaling;
}

function formatLocaleDate(language: string): string {
  const date = new Date();
  if (language.startsWith('ru')) {
    const formatter = new Intl.DateTimeFormat('ru-RU', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
    });
    return formatter.format(date);
  }

  const formatter = new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
  return formatter.format(date);
}

function speciesCitationEntries(referenceGroupId: string): CitationEntry[] {
  if (referenceGroupId.startsWith('cattle_dairy')) {
    return [
      {
        gost: `National Academies of Sciences, Engineering, and Medicine. *Nutrient Requirements of Dairy Cattle: Eighth Revised Edition* [Электронный ресурс]. Washington, DC: The National Academies Press, 2021. URL: ${CATALOG_URLS.dairy} (дата обращения: ${formatLocaleDate('ru')}).`,
        harvard: `National Academies of Sciences, Engineering, and Medicine (2021) *Nutrient Requirements of Dairy Cattle: Eighth Revised Edition*. Washington, DC: The National Academies Press. Available at: ${CATALOG_URLS.dairy} (Accessed: ${formatLocaleDate('en')}).`,
      },
    ];
  }

  if (referenceGroupId.startsWith('cattle_beef')) {
    return [
      {
        gost: `National Research Council. *Nutrient Requirements of Beef Cattle: Eighth Revised Edition* [Электронный ресурс]. Washington, DC: The National Academies Press, 2016. URL: ${CATALOG_URLS.beef} (дата обращения: ${formatLocaleDate('ru')}).`,
        harvard: `National Research Council (2016) *Nutrient Requirements of Beef Cattle: Eighth Revised Edition*. Washington, DC: The National Academies Press. Available at: ${CATALOG_URLS.beef} (Accessed: ${formatLocaleDate('en')}).`,
      },
    ];
  }

  if (referenceGroupId.startsWith('swine')) {
    return [
      {
        gost: `National Research Council. *Nutrient Requirements of Swine: Eleventh Revised Edition* [Электронный ресурс]. Washington, DC: The National Academies Press, 2012. URL: ${CATALOG_URLS.swine} (дата обращения: ${formatLocaleDate('ru')}).`,
        harvard: `National Research Council (2012) *Nutrient Requirements of Swine: Eleventh Revised Edition*. Washington, DC: The National Academies Press. Available at: ${CATALOG_URLS.swine} (Accessed: ${formatLocaleDate('en')}).`,
      },
    ];
  }

  if (referenceGroupId.startsWith('poultry')) {
    return [
      {
        gost: `National Research Council. *Nutrient Requirements of Poultry: Ninth Revised Edition, 1994* [Электронный ресурс]. Washington, DC: National Academy Press, 1994. URL: ${CATALOG_URLS.poultry} (дата обращения: ${formatLocaleDate('ru')}).`,
        harvard: `National Research Council (1994) *Nutrient Requirements of Poultry: Ninth Revised Edition, 1994*. Washington, DC: National Academy Press. Available at: ${CATALOG_URLS.poultry} (Accessed: ${formatLocaleDate('en')}).`,
      },
    ];
  }

  return [];
}

export function getNormMethodologyCopy(
  methodology: NormMethodology | null | undefined,
  t: TFunction,
): MethodologyCopy {
  const keys = methodologyCopyKey(methodology);
  return {
    title: t(keys.title),
    description: t(keys.description),
    formulaMarkdown: t(keys.formulas),
  };
}

export function getNormMethodologyMetricLabel(key: string, t: TFunction): string {
  return t(METRIC_LABEL_KEYS[key] ?? 'norms.methodologyMetric.generic');
}

export function getNormMethodologyFactorLabel(key: string, t: TFunction): string {
  return t(FACTOR_LABEL_KEYS[key] ?? 'norms.methodologyFactor.generic');
}

export function getNormMethodologyUnitLabel(unit: string, t: TFunction): string {
  return t(UNIT_LABEL_KEYS[unit] ?? 'norms.methodologyUnit.generic');
}

export function formatMethodologyMetricValue(
  value: number | undefined,
  unit: string,
  language: string,
): string {
  if (value === undefined || !Number.isFinite(value)) {
    return '--';
  }

  const digits =
    unit === 'pct' || unit === 'g_kg_feed'
      ? 2
      : unit === 'kg' || unit === 'kg_day' || unit === 'mj_day'
        ? 1
        : 0;

  return new Intl.NumberFormat(language.startsWith('ru') ? 'ru-RU' : 'en-US', {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits,
  }).format(value);
}

export function formatMethodologyMetricPair(
  metric: NormMethodologyMetric,
  language: string,
  t: TFunction,
): { reference: string; current: string; unit: string } {
  return {
    reference: formatMethodologyMetricValue(metric.reference_value, metric.unit, language),
    current: formatMethodologyMetricValue(metric.current_value, metric.unit, language),
    unit: getNormMethodologyUnitLabel(metric.unit, t),
  };
}

export function formatMethodologyFactorValue(
  factor: NormMethodologyFactor,
  language: string,
): string {
  const multiplier = new Intl.NumberFormat(language.startsWith('ru') ? 'ru-RU' : 'en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(factor.value);
  const delta = (factor.value - 1) * 100;
  const deltaLabel = new Intl.NumberFormat(language.startsWith('ru') ? 'ru-RU' : 'en-US', {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
    signDisplay: 'always',
  }).format(delta);
  return `x${multiplier} (${deltaLabel}%)`;
}

export function getNormMethodologyCitationMarkdown(
  methodology: NormMethodology | null | undefined,
  language: string,
): string {
  if (!methodology) {
    return '';
  }

  const entries = speciesCitationEntries(methodology.reference_group_id);
  if (!entries.length) {
    return '';
  }

  return entries
    .map((entry) => `- ${language.startsWith('ru') ? entry.gost : entry.harvard}`)
    .join('\n');
}
