import { useMemo } from 'react';
import { NutrientRow } from './NutrientRow';
import { NutrientSection } from './NutrientSection';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import {
  NUTRIENT_CATEGORIES,
  getNutrientCategoryId,
  type NutrientCategoryId,
} from '@/lib/nutrient-categories';
import { useResolvedNormReference } from '@/lib/resolved-norms';
import {
  buildManagedNutrientRows,
  type ManagedNutrientDisplayRow,
} from '@/lib/nutrient-display';
import { resolveNutrientLanguage } from '@/lib/nutrient-registry';
import { useRationStore } from '@/stores/rationStore';

function emptySections(): Record<NutrientCategoryId, NutrientRowModel[]> {
  return {
    general: [],
    energy: [],
    protein: [],
    amino_acids: [],
    fiber_carbs: [],
    fats: [],
    macrominerals: [],
    trace_minerals: [],
    vitamins: [],
    ratios: [],
  };
}

type NutrientRowModel = ManagedNutrientDisplayRow;

export function NutrientPanel() {
  const { t, i18n } = useTranslationWithFallback();
  const {
    nutrients,
    animalProperties,
    animalCount,
    customNorms,
    activeNormPresetId,
  } = useRationStore();
  const nutrientLanguage = resolveNutrientLanguage(i18n.resolvedLanguage);
  const {
    norms: currentNorms,
    referenceLabel,
    resolvedGroupId,
    normSource,
  } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  const nutrientRows = useMemo<NutrientRowModel[]>(
    () => buildManagedNutrientRows(
      nutrients,
      resolvedGroupId,
      currentNorms,
      nutrientLanguage,
      animalCount,
    ),
    [animalCount, currentNorms, nutrientLanguage, nutrients, resolvedGroupId],
  );

  const sectionRows = useMemo(() => {
    const grouped = emptySections();

    nutrientRows.forEach((row) => {
      grouped[getNutrientCategoryId(row.key)].push(row);
    });

    return grouped;
  }, [nutrientRows]);

  if (!nutrients) {
    return (
      <div className="py-8 text-center text-sm text-[--text-secondary]">
        {t('nutrients.addFeedsToSee')}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3 rounded-[--radius-md] bg-[--bg-surface] p-3 md:grid-cols-4">
        <div className="text-center">
          <div className="text-lg font-semibold text-[--text-primary]">
            {nutrients.total_weight_kg.toFixed(1)}
          </div>
          <div className="text-[10px] text-[--text-secondary]">
            {t('nutrients.perHeadTotal')}
          </div>
        </div>
        <div className="text-center">
          <div className="text-lg font-semibold text-[--text-primary]">
            {nutrients.total_dm_kg.toFixed(1)}
          </div>
          <div className="text-[10px] text-[--text-secondary]">
            {t('nutrients.dmKg')}
          </div>
        </div>
        <div className="text-center">
          <div className="text-lg font-semibold text-[--accent]">
            {(nutrients.total_weight_kg * animalCount).toFixed(1)}
          </div>
          <div className="text-[10px] text-[--text-secondary]">
            {t('nutrients.allHeadsTotal')}
          </div>
        </div>
        <div className="text-center">
          <div className="text-lg font-semibold text-[--text-primary]">
            {animalCount}
          </div>
          <div className="text-[10px] text-[--text-secondary]">
            {t('nutrients.heads')}
          </div>
        </div>
      </div>

      <div className="grid gap-3 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3 text-xs text-[--text-secondary] md:grid-cols-2">
        <div>
          <span className="font-medium text-[--text-primary]">
            {t('norms.productionLevel')}:
          </span>{' '}
          {referenceLabel ?? '—'}
        </div>
        <div>
          <span className="font-medium text-[--text-primary]">
            {t('nutrients.referenceSourceTitle')}:
          </span>{' '}
          {normSource === 'backend'
            ? t('nutrients.referenceSourceBackend')
            : t('nutrients.referenceSourceFallback')}
        </div>
      </div>

      {NUTRIENT_CATEGORIES.map((category) => (
        <NutrientSection
          key={category.id}
          id={category.id}
          titleKey={category.titleKey}
          defaultOpen={category.defaultOpen}
        >
          {sectionRows[category.id].map((row) => (
            <NutrientRow
              key={row.key}
              name={row.label}
              actual={row.actual}
              totalActual={row.totalActual}
              normMin={row.normMin}
              normOpt={row.normTarget}
              normMax={row.normMax}
              unit={row.unit}
              status={row.status}
              targetPercent={row.targetPercent}
              showBar={row.targetPercent !== undefined}
            />
          ))}
        </NutrientSection>
      ))}
    </div>
  );
}
