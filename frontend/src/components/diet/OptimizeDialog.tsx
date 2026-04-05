import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertCircle, CheckCircle2, Loader2, SlidersHorizontal, X } from 'lucide-react';
import { AutoAddedFeedsSection } from './AutoAddedFeedsSection';
import { AlternativesModal } from './AlternativesModal';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { cn } from '@/lib/utils';
import { ensureBackendRation } from '@/lib/backend-ration';
import { feedsApi, rationsApi } from '@/lib/api';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getFeedDisplayNameFromCatalog, resolveFeedLanguage } from '@/lib/feed-display';
import { useResolvedNormReference } from '@/lib/resolved-norms';
import {
  getNutrientDisplayUnit,
  getNutrientLabel,
  getNutrientUnit,
  resolveNutrientLanguage,
} from '@/lib/nutrient-registry';
import {
  getRelaxedTargetDisplay,
  localizeOptimizationReason,
  localizeOptimizationStatus,
  localizeWorkflowNote,
  relaxedTargetSummaryKey,
} from '@/lib/optimization-feedback';
import { getReferenceDriftItems } from '@/lib/reference-audit';
import { getPresetsForGroup, type NormRange } from '@/types/nutrient';
import type { Feed } from '@/types/feed';
import type { DietSolution, OptimizeMode, SolveIntent } from '@/types/ration';
import { dietSolutionToOptimizationResult } from '@/lib/solution-nutrients';
import { resolveAnimalGroupId, useRationStore } from '@/stores/rationStore';
import {
  ensureAlternativeFeedCatalog,
  mergeAlternativeIntoDietSolution,
  persistAlternativeSelection,
} from '@/lib/alternative-selection';

interface OptimizeDialogProps {
  onClose: () => void;
}

interface NormOption {
  id: string;
  label: string;
  description: string;
  norms: Record<string, NormRange>;
}

interface LocalOptimizeItem {
  id: string;
  feed: Feed;
  amount_kg: number;
  is_locked: boolean;
}

interface IntentOptionConfig {
  intent: SolveIntent;
  titleKey: string;
  descriptionKey: string;
}

function defaultSolveIntent(feedCount: number): SolveIntent {
  if (feedCount === 0) {
    return 'build_from_library';
  }
  if (feedCount < 3) {
    return 'complete_from_library';
  }
  return 'selected_only';
}

function formatValue(value: number | undefined) {
  if (value === undefined) {
    return '-';
  }
  if (Math.abs(value) >= 100) {
    return value.toFixed(0);
  }
  if (Math.abs(value) >= 10) {
    return value.toFixed(1);
  }
  return value.toFixed(2);
}

function formatRange(range: NormRange | undefined, unit: string) {
  if (!range) {
    return `- ${unit}`;
  }

  if (range.min !== undefined && range.max !== undefined) {
    return `${formatValue(range.min)}-${formatValue(range.max)} ${unit}`;
  }
  if (range.target !== undefined) {
    return `${formatValue(range.target)} ${unit}`;
  }
  if (range.min !== undefined) {
    return `>= ${formatValue(range.min)} ${unit}`;
  }
  if (range.max !== undefined) {
    return `<= ${formatValue(range.max)} ${unit}`;
  }
  return `- ${unit}`;
}

function buildConstraintPreview(
  norms: Record<string, NormRange>,
  groupId: string,
  language: 'ru' | 'en',
) {
  const priority = [
    'dry_matter_intake',
    'feed_intake',
    'energy_eke',
    'energy_oe_pig',
    'energy_oe_poultry',
    'crude_protein',
    'crude_protein_pct',
    'dig_protein_cattle',
    'crude_fiber',
    'ca_p_ratio',
    'calcium',
    'calcium_pct',
    'phosphorus',
    'lysine',
    'lysine_sid',
    'lysine_sid_pct',
    'lysine_tid_pct',
    'methionine_cystine_lys_ratio',
  ];

  return priority
    .filter((key) => norms[key])
    .slice(0, 5)
    .map((key) => {
      return {
        key,
        label: getNutrientLabel(key, language),
        value: formatRange(
          norms[key],
          getNutrientDisplayUnit(groupId, key, language, getNutrientUnit(key, language)),
        ),
      };
    });
}

function buildPresetDescription(
  params: {
    weight?: number;
    milkYield?: number;
    dailyGain?: number;
    eggProduction?: number;
    age?: string;
  },
  t: (key: string) => string,
) {
  const parts: string[] = [];

  if (params.weight) {
    parts.push(`${t('norms.weight')}: ${formatValue(params.weight)} ${t('units.kg')}`);
  }
  if (params.milkYield) {
    parts.push(`${t('norms.milkYield')}: ${formatValue(params.milkYield)} ${t('animal.kgDay')}`);
  }
  if (params.dailyGain) {
    parts.push(`${t('norms.dailyGain')}: ${formatValue(params.dailyGain)} ${t('norms.gDay')}`);
  }
  if (params.eggProduction) {
    parts.push(`${t('animal.eggProduction')}: ${formatValue(params.eggProduction)}`);
  }
  if (params.age) {
    parts.push(`${t('norms.age')}: ${params.age}`);
  }

  return parts.join(' | ');
}

function mergeOptimizedItems(
  workingLocalItems: LocalOptimizeItem[],
  backendFeedIdsByLocalId: Map<string, number>,
  solution: DietSolution,
  feedLookup: Map<number, Feed>,
) {
  const optimizedAmounts = new Map(
    solution.items.map((item) => [item.feed_id, item.amount_kg]),
  );
  const localIdByFeedId = new Map(
    Array.from(backendFeedIdsByLocalId.entries()).map(([localId, feedId]) => [feedId, localId]),
  );

  const merged = workingLocalItems.flatMap((item) => {
    const backendFeedId = backendFeedIdsByLocalId.get(item.id);
    if (backendFeedId === undefined) {
      return [item];
    }

    const nextAmount = optimizedAmounts.has(backendFeedId)
      ? optimizedAmounts.get(backendFeedId) ?? 0
      : 0;
    const roundedAmount = Math.round(nextAmount * 1000) / 1000;

    if (roundedAmount <= 0) {
      return [];
    }

    return [{
      ...item,
      amount_kg: roundedAmount,
    }];
  });

  for (const item of solution.items) {
    if (localIdByFeedId.has(item.feed_id)) {
      continue;
    }
    const feed = feedLookup.get(item.feed_id);
    if (!feed) {
      continue;
    }
    const roundedAmount = Math.round(item.amount_kg * 1000) / 1000;
    if (roundedAmount <= 0) {
      continue;
    }

    merged.push({
      id: `optimized-${item.feed_id}`,
      feed,
      amount_kg: roundedAmount,
      is_locked: false,
    });
  }

  return merged;
}

export function OptimizeDialog({ onClose }: OptimizeDialogProps) {
  const { t, i18n } = useTranslation();
  const { feeds: feedCatalog } = useFeedCatalog();
  const nutrientLanguage = resolveNutrientLanguage(i18n.resolvedLanguage);
  const [mode, setMode] = useState<OptimizeMode>('tiered');
  const [intent, setIntent] = useState<SolveIntent>(defaultSolveIntent(useRationStore.getState().localItems.length));
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [result, setResult] = useState<'success' | 'error' | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [solution, setSolution] = useState<DietSolution | null>(null);

  const {
    localItems,
    setLocalItems,
    animalProperties,
    animalCount,
    currentRationId,
    currentProjectName,
    setCurrentRation,
    customNorms,
    activeNormPresetId,
    setActiveNormPreset,
    setOptimizationFeedback,
    nutrients,
    alternatives,
    alternativesCurrentIndex,
    alternativesShowModal,
    alternativesPendingApply,
    setAlternatives,
    selectAlternative,
    toggleAlternativesModal,
    clearAlternatives,
    farmBucket,
    farmBucketActive,
  } = useRationStore();

  const animalGroupId = resolveAnimalGroupId(animalProperties);
  const {
    norms: currentNorms,
    referenceLabel: autoReferenceLabel,
    resolvedGroupId,
  } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  const normOptions = useMemo<NormOption[]>(() => {
    const presets = getPresetsForGroup(animalGroupId);
    const currentDescription = customNorms && Object.keys(customNorms).length > 0
      ? t('optimize.currentReferenceManual')
      : autoReferenceLabel || t('optimize.currentReferenceAuto');

    return [
      {
        id: 'current',
        label: t('optimize.currentReference'),
        description: currentDescription,
        norms: currentNorms,
      },
      ...presets.map((preset) => ({
        id: preset.id,
        label: nutrientLanguage === 'en' ? preset.label_en : preset.label_ru,
        description: buildPresetDescription(preset.params, t),
        norms: preset.norms,
      })),
    ];
  }, [animalGroupId, autoReferenceLabel, currentNorms, customNorms, nutrientLanguage, t]);

  const [selectedReferenceId, setSelectedReferenceId] = useState<string>(
    activeNormPresetId && normOptions.some((option) => option.id === activeNormPresetId)
      ? activeNormPresetId
      : 'current',
  );

  const selectedReference = normOptions.find((option) => option.id === selectedReferenceId) ?? normOptions[0];
  const constraints = useMemo(
    () => buildConstraintPreview(selectedReference?.norms ?? currentNorms, resolvedGroupId, nutrientLanguage),
    [currentNorms, nutrientLanguage, resolvedGroupId, selectedReference],
  );
  const referenceDrift = useMemo(
    () => (
      nutrients
        ? getReferenceDriftItems(
          resolvedGroupId,
          nutrients,
          selectedReference?.norms ?? currentNorms,
        ).slice(0, 4)
        : []
    ),
    [currentNorms, nutrients, resolvedGroupId, selectedReference],
  );
  const solutionTone = solution?.best_achievable ? 'warning' : 'success';
  const availableModes = useMemo<OptimizeMode[]>(
    () => (intent === 'selected_only'
      ? ['tiered', 'single_pass', 'minimize_cost', 'fixed']
      : ['repair', 'tiered']),
    [intent],
  );
  const massUnit = i18n.language.startsWith('ru') ? 'кг' : 'kg';
  const feedLanguage = resolveFeedLanguage(i18n.resolvedLanguage);
  const intentOptions: IntentOptionConfig[] = [
    {
      intent: 'selected_only',
      titleKey: 'optimize.intentSelectedOnly',
      descriptionKey: 'optimize.intentSelectedOnlyDesc',
    },
    {
      intent: 'complete_from_library',
      titleKey: 'optimize.intentCompleteLibrary',
      descriptionKey: 'optimize.intentCompleteLibraryDesc',
    },
    {
      intent: 'build_from_library',
      titleKey: 'optimize.intentBuildLibrary',
      descriptionKey: 'optimize.intentBuildLibraryDesc',
    },
  ];

  useEffect(() => {
    if (!availableModes.includes(mode)) {
      setMode(availableModes[0]);
    }
  }, [availableModes, mode]);

  const handleAlternativesClose = () => {
    if (alternativesPendingApply) {
      clearAlternatives();
      return;
    }
    if (alternativesShowModal) {
      toggleAlternativesModal();
    }
  };

  const handleDialogClose = () => {
    if (alternativesPendingApply) {
      clearAlternatives();
    }
    onClose();
  };

  const handleApplyAlternative = async (index: number) => {
    const selected = alternatives[index];
    if (!selected) {
      return;
    }

    const rationId = currentRationId ?? useRationStore.getState().currentRationId;
    if (!rationId) {
      setErrorMessage(t('optimize.errorMessage'));
      setResult('error');
      return;
    }

    try {
      const resolvedFeedCatalog = await ensureAlternativeFeedCatalog(selected, feedCatalog);
      await persistAlternativeSelection(rationId, selected);
      selectAlternative(index, resolvedFeedCatalog);

      const nextFeedback = mergeAlternativeIntoDietSolution(
        solution ?? useRationStore.getState().optimizationFeedback?.solution,
        selected,
      );
      if (nextFeedback) {
        setOptimizationFeedback(nextFeedback);
        setSolution(nextFeedback);
      }

      if (selectedReference.id !== 'current') {
        setActiveNormPreset(selectedReference.id);
      }

      setResult('success');

      if (alternativesPendingApply) {
        if (alternativesShowModal) {
          toggleAlternativesModal();
        }
        onClose();
      }
    } catch (err) {
      setResult('error');
      setErrorMessage(err instanceof Error ? err.message : t('optimize.errorMessage'));
    }
  };

  const handleOptimize = async () => {
    setIsOptimizing(true);
    setResult(null);
    setErrorMessage(null);
    setSolution(null);

    try {
      const { rationId, backendFeedIdsByLocalId: initialBackendFeedIds } = await ensureBackendRation({
        currentRationId,
        animalGroupId,
        animalCount,
        currentProjectName,
        localItems,
      });

      setCurrentRation(rationId, animalGroupId);

      let workingLocalItems = localItems;
      let backendFeedIdsByLocalId = initialBackendFeedIds;
      const optimizePayload = {
        mode,
        intent,
        norms: selectedReference.norms,
        norm_preset_id:
          selectedReference.id === 'current' ? activeNormPresetId : selectedReference.id,
        animal_properties: {
          species: animalProperties.species,
          production_type: animalProperties.productionType,
          breed: animalProperties.breed,
          sex: animalProperties.sex,
          live_weight_kg: animalProperties.liveWeightKg,
          age_from_days: animalProperties.ageFromDays,
          age_to_days: animalProperties.ageToDays,
          milk_yield_kg: animalProperties.milkYieldKg,
          milk_fat_pct: animalProperties.milkFatPct,
          daily_gain_g: animalProperties.dailyGainG,
          egg_production_per_year: animalProperties.eggProductionPerYear,
          litter_size: animalProperties.litterSize,
          reproductive_stage: animalProperties.reproductiveStage,
        },
        available_feed_ids: farmBucketActive && farmBucket.size > 0 ? [...farmBucket] : undefined,
      };

      const response = await rationsApi.optimize(rationId, optimizePayload);
      const nextSolution = response.data;
      const hasFeasibleSolution = ['Optimal', 'Feasible'].includes(nextSolution.optimization_status);

      if (!hasFeasibleSolution) {
        setOptimizationFeedback(null);
        setSolution(nextSolution);
        setResult('error');
        const workflowMessage = nextSolution.workflow_notes?.[nextSolution.workflow_notes.length - 1];
        setErrorMessage(
          (workflowMessage ? localizeWorkflowNote(workflowMessage, t) : null) || (
            nextSolution.optimization_status === 'Infeasible'
            ? t('optimize.noSolution')
            : t('optimize.errorMessage')
          ),
        );
        return;
      }

      if (intent === 'build_from_library') {
        let proposedResult = dietSolutionToOptimizationResult(nextSolution);
        try {
          const alternativesResponse = await rationsApi.optimizeAlternatives(rationId, optimizePayload);
          proposedResult = alternativesResponse.data;
        } catch {
          // Fall back to the alternatives attached to the optimize response.
        }

        if (proposedResult.alternatives.length > 0) {
          setSolution(nextSolution);
          setResult('success');
          setAlternatives(proposedResult, { pendingApply: true });
          return;
        }
      }

      // Always fetch full feed list: backend may have auto-added feeds via
      // auto-populate or auto-repair that the frontend doesn't know about
      const backendFeeds = (await feedsApi.list({ limit: 5000 })).data ?? [];
      const feedLookup = new Map<number, Feed>(backendFeeds.map((feed) => [feed.id, feed]));
      // Also add known local feeds
      for (const item of workingLocalItems) {
        if (!feedLookup.has(item.feed.id)) {
          feedLookup.set(item.feed.id, item.feed);
        }
      }

      const optimizedItems = mergeOptimizedItems(
        workingLocalItems,
        backendFeedIdsByLocalId,
        nextSolution,
        feedLookup,
      );
      for (const item of optimizedItems) {
        backendFeedIdsByLocalId.set(item.id, item.feed.id);
      }

      if (selectedReference.id !== 'current') {
        setActiveNormPreset(selectedReference.id);
      }

      await rationsApi.update(rationId, {
        items: optimizedItems.map((item) => ({
          feed_id: backendFeedIdsByLocalId.get(item.id) ?? item.feed.id,
          amount_kg: item.amount_kg,
          is_locked: item.is_locked,
        })),
      });
      setLocalItems(optimizedItems);
      setOptimizationFeedback(nextSolution);
      setSolution(nextSolution);
      setResult('success');

      const optimizationResult = dietSolutionToOptimizationResult(nextSolution);
      setAlternatives(optimizationResult);
    } catch (err) {
      setOptimizationFeedback(null);
      setResult('error');
      setErrorMessage(err instanceof Error ? err.message : t('optimize.errorMessage'));
    } finally {
      setIsOptimizing(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50" onClick={handleDialogClose} />

        <div className="relative mx-4 flex max-h-[90vh] w-full max-w-xl flex-col overflow-hidden rounded-[--radius-lg] border border-[--border] bg-[--bg-base] shadow-xl">
          <div className="flex shrink-0 items-center justify-between border-b border-[--border] px-4 py-3">
            <div className="flex items-center gap-2">
              <Icon icon={SlidersHorizontal} size={16} className="text-[--text-secondary]" />
              <h2 className="text-sm font-medium text-[--text-primary]">{t('optimize.title')}</h2>
            </div>
          <button
            onClick={handleDialogClose}
            className="rounded p-1 text-[--text-secondary] transition-colors hover:bg-[--bg-hover]"
          >
            <Icon icon={X} size={16} />
          </button>
        </div>

        <div className="flex-1 space-y-4 overflow-y-auto p-4">
          <p className="text-xs text-[--text-secondary]">{t('optimize.description')}</p>
          {localItems.length === 0 ? (
            <p className="rounded-[--radius-md] bg-[--bg-surface] px-3 py-2 text-[10px] text-[--text-secondary]">
              {t('optimize.emptyStarterHint')}
            </p>
          ) : null}

          <div className="space-y-2">
            <div className="mb-1 text-xs font-medium text-[--text-secondary]">
              {t('optimize.intentTitle')}
            </div>
            {intentOptions.map((option) => (
              <IntentOption
                key={option.intent}
                intent={option.intent}
                currentIntent={intent}
                onSelect={setIntent}
                title={t(option.titleKey)}
                description={t(option.descriptionKey)}
              />
            ))}
          </div>

          <div className="space-y-2">
            <div className="mb-1 text-xs font-medium text-[--text-secondary]">
              {t('optimize.strategyTitle')}
            </div>
            {availableModes.includes('tiered') ? (
              <ModeOption
                mode="tiered"
                currentMode={mode}
                onSelect={setMode}
                title={t('optimize.tieredBalance')}
                description={t('optimize.tieredBalanceDesc')}
              />
            ) : null}
            {availableModes.includes('repair') ? (
              <ModeOption
                mode="repair"
                currentMode={mode}
                onSelect={setMode}
                title={t('optimize.repairBalance')}
                description={t('optimize.repairBalanceDesc')}
              />
            ) : null}
            {availableModes.includes('single_pass') ? (
              <ModeOption
                mode="single_pass"
                currentMode={mode}
                onSelect={setMode}
                title={t('optimize.quickBalance')}
                description={t('optimize.quickBalanceDesc')}
              />
            ) : null}
            {availableModes.includes('minimize_cost') ? (
              <ModeOption
                mode="minimize_cost"
                currentMode={mode}
                onSelect={setMode}
                title={t('optimize.minimizeCost')}
                description={t('optimize.minimizeCostDesc')}
              />
            ) : null}
            {availableModes.includes('fixed') ? (
              <ModeOption
                mode="fixed"
                currentMode={mode}
                onSelect={setMode}
                title={t('optimize.fixedFeeds')}
                description={t('optimize.fixedFeedsDesc')}
              />
            ) : null}
          </div>

          {referenceDrift.length > 0 ? (
            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-secondary]">
              <div className="font-medium text-[--text-primary]">{t('optimize.referenceDriftTitle')}</div>
              <div className="mt-1">{t('optimize.referenceDriftDesc')}</div>
              <div className="mt-2 space-y-1 text-[10px]">
                {referenceDrift.map((item) => (
                  <div key={item.key}>
                    {i18n.language.startsWith('ru') ? item.name_ru : item.name_en}: {' '}
                    {item.direction === 'high'
                      ? t('optimize.referenceDriftHigh')
                      : t('optimize.referenceDriftLow')}
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          <div className="grid gap-3 md:grid-cols-[1.1fr_0.9fr]">
            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <label className="mb-2 block text-xs font-medium text-[--text-secondary]">
                {t('optimize.referenceSet')}
              </label>
              <select
                value={selectedReferenceId}
                onChange={(event) => setSelectedReferenceId(event.target.value)}
                className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs text-[--text-primary]"
              >
                {normOptions.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.label}
                  </option>
                ))}
              </select>
              <p className="mt-2 text-[10px] text-[--text-secondary]">
                {selectedReference.description || t('optimize.currentReferenceAuto')}
              </p>
            </div>

            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <div className="mb-1 text-xs font-medium text-[--text-secondary]">
                {t('optimize.herdSize')}
              </div>
              <div className="text-lg font-semibold text-[--text-primary]">{animalCount}</div>
              <div className="mt-1 text-[10px] text-[--text-secondary]">
                {t('optimize.herdSizeHint')}
              </div>
            </div>
          </div>

          <div className="rounded-[--radius-md] bg-[--bg-surface] p-3">
            <h4 className="mb-2 text-xs font-medium text-[--text-secondary]">
              {t('optimize.constraints')}
            </h4>
            <ul className="space-y-1 text-xs text-[--text-secondary]">
              {constraints.map((constraint) => (
                <li key={constraint.key} className="flex items-center gap-2">
                  <span className="h-1.5 w-1.5 rounded-full bg-[--status-ok]" />
                  {constraint.label}: {constraint.value}
                </li>
              ))}
            </ul>
          </div>

          {result === 'success' && solution ? (
            <div className={cn(
              'flex items-start gap-2 rounded-[--radius-md] p-3',
              solutionTone === 'warning' ? 'bg-[--status-warn-bg]' : 'bg-[--status-ok-bg]',
            )}>
              <Icon
                icon={solutionTone === 'warning' ? AlertCircle : CheckCircle2}
                size={16}
                className={cn(
                  'mt-0.5',
                  solutionTone === 'warning' ? 'text-[--status-warn]' : 'text-[--status-ok]',
                )}
              />
              <div className={cn(
                'text-xs',
                solutionTone === 'warning' ? 'text-[--status-warn]' : 'text-[--status-ok]',
              )}>
                <div className="font-medium">
                  {solution.best_achievable
                    ? t('optimize.bestAchievableMessage')
                    : t('optimize.successMessage')}
                </div>
                <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 opacity-80">
                  <span>
                    {t('optimize.resultStatusLabel')}: {localizeOptimizationStatus(solution.optimization_status, t)}
                  </span>
                  <span>
                    {t('optimize.resultCostLabel')}: {solution.cost_per_day.toFixed(2)} / {t('optimize.perHead')}
                  </span>
                </div>
              </div>
            </div>
          ) : null}

          {solution?.relaxed_targets?.length ? (
            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <h4 className="mb-2 text-xs font-medium text-[--text-secondary]">
                {t('optimize.relaxedTargets')}
              </h4>
              <ul className="space-y-2 text-xs text-[--text-secondary]">
                {solution.relaxed_targets.map((target) => {
                  const meta = getRelaxedTargetDisplay(target, i18n.language);
                  return (
                    <li key={`${target.key}-${target.constraint_type}`} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
                      <div className="font-medium text-[--text-primary]">
                        {meta.label}
                      </div>
                      <div>
                        {t(relaxedTargetSummaryKey(target.constraint_type))}: {formatValue(target.actual)} {meta.unit}
                        {meta.unit ? ' ' : ''}
                        / {formatValue(target.target)} {meta.unit}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </div>
          ) : null}

          {solution?.workflow_notes?.length ? (
            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <h4 className="mb-2 text-xs font-medium text-[--text-secondary]">
                {t('optimize.workflowSummary')}
              </h4>
              <ul className="space-y-2 text-xs text-[--text-secondary]">
                {solution.workflow_notes.map((note) => (
                  <li key={note} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
                    {localizeWorkflowNote(note, t)}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          {solution?.auto_added_feeds?.length ? (
            <AutoAddedFeedsSection
              feeds={solution.auto_added_feeds}
              titleKey="optimize.autoAddedFeeds"
            />
          ) : null}

          {solution?.recommendations?.length ? (
            <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <h4 className="mb-2 text-xs font-medium text-[--text-secondary]">
                {t('optimize.recommendations')}
              </h4>
              <ul className="space-y-2 text-xs text-[--text-secondary]">
                {solution.recommendations.slice(0, 3).map((recommendation) => (
                  <li key={recommendation.feed_id} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
                    <div className="font-medium text-[--text-primary]">
                      {getFeedDisplayNameFromCatalog(recommendation.feed_id, recommendation.feed_name, feedCatalog, feedLanguage)} ({recommendation.suggested_amount_kg.toFixed(2)} {massUnit})
                    </div>
                    <div>{localizeOptimizationReason(recommendation.reason, t)}</div>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          {result === 'error' ? (
            <div className="flex items-start gap-2 rounded-[--radius-md] bg-[--status-error-bg] p-3">
              <Icon icon={AlertCircle} size={16} className="mt-0.5 text-[--status-error]" />
              <span className="text-xs text-[--status-error]">
                {errorMessage || t('optimize.errorMessage')}
              </span>
            </div>
          ) : null}
        </div>

        <div className="flex shrink-0 items-center justify-end gap-2 border-t border-[--border] px-4 py-3">
          <Button variant="ghost" size="sm" onClick={handleDialogClose}>
            {t('common.cancel')}
          </Button>
          <Button
            size="sm"
            onClick={handleOptimize}
            disabled={isOptimizing}
          >
            {isOptimizing ? (
              <>
                <Icon icon={Loader2} size={14} className="mr-1.5 animate-spin" />
                {t('optimize.optimizing')}
              </>
            ) : (
              <>
                <Icon icon={SlidersHorizontal} size={14} className="mr-1.5" />
                {t('optimize.optimizeBtn')}
              </>
            )}
          </Button>
        </div>
      </div>

      {alternativesShowModal && alternatives.length > 1 && (
        <AlternativesModal
          solutions={alternatives}
          currentIndex={alternativesCurrentIndex}
          groupId={resolvedGroupId}
          norms={selectedReference?.norms ?? currentNorms}
          pendingApply={alternativesPendingApply}
          onSelect={(index) => { void handleApplyAlternative(index); }}
          onClose={handleAlternativesClose}
        />
      )}
    </div>
  );
}

interface ModeOptionProps {
  mode: OptimizeMode;
  currentMode: OptimizeMode;
  onSelect: (mode: OptimizeMode) => void;
  title: string;
  description: string;
}

interface IntentOptionProps {
  intent: SolveIntent;
  currentIntent: SolveIntent;
  onSelect: (intent: SolveIntent) => void;
  title: string;
  description: string;
}

function IntentOption({ intent, currentIntent, onSelect, title, description }: IntentOptionProps) {
  const isSelected = intent === currentIntent;

  return (
    <button
      onClick={() => onSelect(intent)}
      className={cn(
        'w-full rounded-[--radius-md] border p-3 text-left transition-colors',
        isSelected
          ? 'border-[--accent] bg-[--bg-active]'
          : 'border-[--border] bg-[--bg-surface] hover:border-[--text-disabled]'
      )}
    >
      <div className="text-xs font-medium text-[--text-primary]">{title}</div>
      <div className="mt-1 text-[10px] text-[--text-secondary]">{description}</div>
    </button>
  );
}

function ModeOption({ mode, currentMode, onSelect, title, description }: ModeOptionProps) {
  const isSelected = mode === currentMode;

  return (
    <button
      onClick={() => onSelect(mode)}
      className={cn(
        'w-full rounded-[--radius-md] border p-3 text-left transition-colors',
        isSelected
          ? 'border-[--accent] bg-[--bg-active]'
          : 'border-[--border] bg-[--bg-surface] hover:border-[--text-disabled]'
      )}
    >
      <div className="mb-1 flex items-center gap-2">
        <span
          className={cn(
            'flex h-3 w-3 items-center justify-center rounded-full border-2',
            isSelected ? 'border-[--accent]' : 'border-[--text-disabled]'
          )}
        >
          {isSelected ? <span className="h-1.5 w-1.5 rounded-full bg-[--accent]" /> : null}
        </span>
        <span className="text-xs font-medium text-[--text-primary]">{title}</span>
      </div>
      <p className="ml-5 text-[10px] text-[--text-secondary]">{description}</p>
    </button>
  );
}
