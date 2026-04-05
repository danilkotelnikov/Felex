import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { useQuery } from '@tanstack/react-query';
import {
  Beef,
  Bird,
  PiggyBank,
  Calculator,
  BarChart3,
  DollarSign,
  FileText,
  Sparkles,
  Plus,
  Home,
  BookOpen,
  ChevronDown,
  ChevronUp,
  RotateCcw,
} from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { cn, formatCurrency, formatNumber } from '@/lib/utils';
import { RationTable } from '../diet/RationTable';
import { RationAppliedChangesPanel } from '../diet/RationAppliedChangesPanel';
import { AlternativesPanel } from '../diet/AlternativesPanel';
import { NutrientPanel } from '../diet/NutrientPanel';
import { EconomicsPanel } from '../diet/EconomicsPanel';
import { OptimizationFeedbackPanel } from '../diet/OptimizationFeedbackPanel';
import { OptimizeDialog } from '../diet/OptimizeDialog';
import { AlternativesModal } from '../diet/AlternativesModal';
import { AnimalPropertyEditor, AnimalPropertySummary } from '../animal/AnimalPropertyEditor';
import { SettingsPanel } from '../settings/SettingsPanel';
import { PricesPanel } from '../prices/PricesPanel';
import { NormMethodologyCard } from '../norms/NormMethodologyCard';
import { FeedRecommendations } from '../dashboard/FeedRecommendations';
import { PresetSelector } from '../dashboard/PresetSelector';
import {
  calculateLocalNutrients,
  getDefaultAnimalProperties,
  getAnimalPropertiesForPreset,
  getGroupIdForPreset,
  resolveAnimalGroupId,
  useRationStore,
} from '@/stores/rationStore';
import type { Feed } from '@/types/feed';
import { getPresetsForGroup, type NormRange } from '@/types/nutrient';
import type { AnimalProperties } from '@/stores/rationStore';
import type { RationProject } from '@/types/ration-project';
import type { AlternativeRationSolution } from '@/types/optimization';
import { RATION_PRESETS } from '@/data/ration-presets';
import { workspaceApi } from '@/lib/workspace-api';
import { requestWorkspaceRefresh } from '@/lib/workspace-events';
import { findFeedByKnownName, useFeedCatalog } from '@/lib/feed-catalog';
import {
  presetsApi,
  type MatchedPresetSubcategory,
} from '@/lib/api';
import { getNutrientLabel, getNutrientUnit, getOrderedNutrientKeys } from '@/lib/nutrient-registry';
import {
  exportCsv,
  exportExcel,
  exportPdf,
  loadExportPreferences,
  saveExportPreferences,
} from '@/lib/export';
import { useResolvedNormReference } from '@/lib/resolved-norms';
import { getBaseNormsForGroupId } from '@/lib/norms';
import { capitalizeFirst } from '@/lib/text-utils';
import {
  ensureAlternativeFeedCatalog,
  mergeAlternativeIntoDietSolution,
  persistAlternativeSelection,
} from '@/lib/alternative-selection';

type Tab = 'ration' | 'nutrients' | 'economics' | 'report';

const NORM_PRESET_BY_RATION_PRESET: Record<string, string> = {
  dairy_25: 'cattle_dairy_25',
  dairy_35: 'cattle_dairy_35',
  beef_400: 'cattle_beef_400',
  beef_550: 'cattle_beef_500',
  swine_starter: 'swine_starter',
  swine_finisher: 'swine_finisher_preset',
  swine_sow_gestation: 'swine_sow_gestation',
  swine_sow_lactation: 'swine_sow_lactation',
  broiler_starter: 'poultry_broiler_starter',
  broiler_finisher: 'poultry_broiler_finisher',
  layer_phase1: 'poultry_layer_phase1',
  layer_phase2: 'poultry_layer_phase2',
};

function matchesKnownFeedName(feed: Feed, candidateName: string | null | undefined): boolean {
  if (!candidateName?.trim()) {
    return false;
  }

  return Boolean(findFeedByKnownName([feed], candidateName));
}

function resolveCanonicalFeed(catalog: Feed[], feed: Feed): Feed | null {
  if (feed.source_id) {
    const bySourceId = catalog.find((candidate) => candidate.source_id === feed.source_id);
    if (bySourceId) {
      return bySourceId;
    }
  }

  const byId = catalog.find((candidate) => candidate.id === feed.id);
  if (
    byId &&
    (matchesKnownFeedName(byId, feed.name_ru) || matchesKnownFeedName(byId, feed.name_en))
  ) {
    return byId;
  }

  return (
    findFeedByKnownName(catalog, feed.name_ru) ??
    (feed.name_en ? findFeedByKnownName(catalog, feed.name_en) : undefined) ??
    null
  );
}

function shouldRefreshFeedSnapshot(current: Feed, canonical: Feed): boolean {
  return (
    current.id !== canonical.id ||
    current.source_id !== canonical.source_id ||
    current.name_ru !== canonical.name_ru ||
    current.name_en !== canonical.name_en ||
    current.category !== canonical.category ||
    current.subcategory !== canonical.subcategory ||
    current.price_per_ton !== canonical.price_per_ton ||
    current.price_updated_at !== canonical.price_updated_at ||
    current.region !== canonical.region
  );
}

export function MainWorkspace() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<Tab>('ration');
  const [showOptimize, setShowOptimize] = useState(false);
  const [showPropertyEditor, setShowPropertyEditor] = useState(false);
  const {
    currentRationId,
    localItems,
    animalCount,
    activeNormPresetId,
    animalProperties,
    currentProjectPath,
    currentProjectName,
    currentProjectCreatedAt,
    customNorms,
    setAnimalProperties,
    setLocalItems,
    setOptimizationFeedback,
    activeView,
    optimizationFeedback,
    alternatives,
    alternativesCurrentIndex,
    alternativesShowModal,
    alternativesPanelExpanded,
    alternativesPendingApply,
    selectAlternative,
    toggleAlternativesModal,
    toggleAlternativesPanel,
  } = useRationStore();

  const { feeds: feedCatalog, usingFallback: usingFallbackFeedCatalog } = useFeedCatalog();

  const { norms: currentNorms, resolvedGroupId } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  useEffect(() => {
    if (usingFallbackFeedCatalog || feedCatalog.length === 0 || localItems.length === 0) {
      return;
    }

    const nextItems = localItems.map((item) => {
      const canonicalFeed = resolveCanonicalFeed(feedCatalog, item.feed);
      if (!canonicalFeed || !shouldRefreshFeedSnapshot(item.feed, canonicalFeed)) {
        return item;
      }

      return {
        ...item,
        feed: canonicalFeed,
      };
    });

    const changed = nextItems.some((item, index) => item !== localItems[index]);
    if (changed) {
      setLocalItems(nextItems);
    }
  }, [feedCatalog, localItems, setLocalItems, usingFallbackFeedCatalog]);

  useEffect(() => {
    if (!currentProjectPath || !currentProjectName) {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      const project: RationProject = {
        version: '1.0',
        name: currentProjectName,
        animalGroupId: resolveAnimalGroupId(animalProperties),
        animalProperties: {
          species: animalProperties.species,
          productionType: animalProperties.productionType,
          breed: animalProperties.breed,
          sex: animalProperties.sex,
          ageMonths: animalProperties.ageToDays
            ? Math.round((animalProperties.ageToDays / 30) * 10) / 10
            : animalProperties.ageFromDays
              ? Math.round((animalProperties.ageFromDays / 30) * 10) / 10
              : undefined,
          weight: animalProperties.liveWeightKg,
          milkYieldKg: animalProperties.milkYieldKg,
          milkFatPercent: animalProperties.milkFatPct,
          dailyGainG: animalProperties.dailyGainG,
          eggProduction: animalProperties.eggProductionPerYear,
          litterSize: animalProperties.litterSize,
          stage: animalProperties.reproductiveStage,
        },
        animalCount,
        items: localItems.map((item) => ({
          feedId: item.feed.id,
          feedName: item.feed.name_ru,
          amountKg: item.amount_kg,
          isLocked: item.is_locked,
        })),
        normPresetId: activeNormPresetId ?? undefined,
        customNorms: customNorms ?? undefined,
        createdAt: currentProjectCreatedAt ?? new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      void workspaceApi.updateRation(currentProjectPath, project).catch((error) => {
        console.error('Failed to autosave workspace ration', error);
      });
    }, 400);

    return () => window.clearTimeout(timeoutId);
  }, [
    activeNormPresetId,
    animalCount,
    animalProperties,
    currentProjectCreatedAt,
    currentProjectName,
    currentProjectPath,
    customNorms,
    localItems,
  ]);

  if (activeView === 'settings') {
    return (
      <main className="flex-1 min-w-0 flex flex-col overflow-hidden bg-[--bg-base]">
        <SettingsPanel />
      </main>
    );
  }

  if (activeView === 'prices') {
    return (
      <main className="flex-1 min-w-0 flex flex-col overflow-hidden bg-[--bg-base]">
        <PricesPanel />
      </main>
    );
  }

  if (activeView === 'dashboard') {
    return <DashboardView />;
  }

  if (activeView === 'norms') {
    return <NormsView />;
  }

  const tabs: { id: Tab; labelKey: string; icon: typeof Calculator }[] = [
    { id: 'ration', labelKey: 'workspace.ration', icon: Calculator },
    { id: 'nutrients', labelKey: 'workspace.nutrients', icon: BarChart3 },
    { id: 'economics', labelKey: 'workspace.economics', icon: DollarSign },
    { id: 'report', labelKey: 'workspace.report', icon: FileText },
  ];

  const handleSaveProperties = (props: AnimalProperties) => {
    setAnimalProperties(props);
  };

  const handleApplyAlternative = async (index: number) => {
    const selected = alternatives[index];
    if (!selected) {
      return;
    }

    const rationId = currentRationId ?? useRationStore.getState().currentRationId;

    try {
      const resolvedFeedCatalog = await ensureAlternativeFeedCatalog(selected, feedCatalog);
      if (rationId) {
        await persistAlternativeSelection(rationId, selected);
      }

      selectAlternative(index, resolvedFeedCatalog);

      const nextFeedback = mergeAlternativeIntoDietSolution(
        optimizationFeedback?.solution,
        selected,
      );
      if (nextFeedback) {
        setOptimizationFeedback(nextFeedback);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('optimize.errorMessage'));
    }
  };

  const getGroupDisplayName = () => {
    const speciesNames: Record<string, string> = {
      cattle: t('nav.cattle'),
      swine: t('nav.swine'),
      poultry: t('nav.poultry'),
    };

    const typeNames: Record<string, string> = {
      dairy: t('nav.dairyCows'),
      beef: t('nav.beefCattle'),
      fattening: t('nav.finishers'),
      breeding: t('nav.sows'),
      broiler: t('nav.broilers'),
      layer: t('nav.layers'),
    };

    return typeNames[animalProperties.productionType] || speciesNames[animalProperties.species] || t('animal.freshCows');
  };

  const getSpeciesIcon = () => {
    switch (animalProperties.species) {
      case 'swine':
        return PiggyBank;
      case 'poultry':
        return Bird;
      default:
        return Beef;
    }
  };

  return (
    <main className="flex-1 min-w-0 flex flex-col overflow-hidden bg-[--bg-base]">
      <header className="px-4 py-3 border-b border-[--border]">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <Icon icon={getSpeciesIcon()} size={20} className="text-[--accent]" />
            <div className="min-w-0">
              <h1 className="text-sm font-medium text-[--text-primary] truncate">{getGroupDisplayName()}</h1>
              <AnimalPropertySummary properties={animalProperties} onEdit={() => setShowPropertyEditor(true)} />
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Button variant="outline" size="sm" onClick={() => setShowOptimize(true)}>
              <Icon icon={Sparkles} size={14} className="mr-1.5" />
              {t('workspace.optimize')}
            </Button>
          </div>
        </div>
      </header>

      <div className="flex items-center gap-1 px-4 py-2 border-b border-[--border]">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => { setActiveTab(tab.id); if (alternativesShowModal) toggleAlternativesModal(); }}
            className={cn(
              'flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-[--radius-sm] transition-colors',
              activeTab === tab.id
                ? 'bg-[--bg-active] text-[--accent]'
                : 'text-[--text-secondary] hover:bg-[--bg-hover] hover:text-[--text-primary]'
            )}
          >
            <Icon icon={tab.icon} size={14} />
            {t(tab.labelKey)}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4">
        <OptimizationFeedbackPanel onOpenOptimize={() => setShowOptimize(true)} />
        {activeTab === 'ration' ? (
          <RationTab
            alternatives={alternatives}
            alternativesCurrentIndex={alternativesCurrentIndex}
            alternativesExpanded={alternativesPanelExpanded}
            groupId={resolvedGroupId}
            norms={currentNorms}
            onSelectAlternative={(index) => { void handleApplyAlternative(index); }}
            onToggleAlternatives={toggleAlternativesPanel}
            onOpenAlternativesWindow={() => {
              if (!alternativesShowModal) {
                toggleAlternativesModal();
              }
            }}
          />
        ) : null}
        {activeTab === 'nutrients' ? <NutrientsTab /> : null}
        {activeTab === 'economics' ? <EconomicsTab /> : null}
        {activeTab === 'report' ? <ReportTab /> : null}
      </div>

      {showOptimize ? <OptimizeDialog onClose={() => setShowOptimize(false)} /> : null}
      {showPropertyEditor ? (
        <AnimalPropertyEditor
          properties={animalProperties}
          onSave={handleSaveProperties}
          onClose={() => setShowPropertyEditor(false)}
        />
      ) : null}
      {alternativesShowModal && alternatives.length > 1 && !showOptimize ? (
        <AlternativesModal
          solutions={alternatives}
          currentIndex={alternativesCurrentIndex}
          groupId={resolvedGroupId}
          norms={currentNorms}
          pendingApply={alternativesPendingApply}
          onSelect={(index) => { void handleApplyAlternative(index); }}
          onClose={toggleAlternativesModal}
        />
      ) : null}
    </main>
  );
}

interface RationTabProps {
  alternatives: AlternativeRationSolution[];
  alternativesCurrentIndex: number;
  alternativesExpanded: boolean;
  groupId: string;
  norms: Record<string, NormRange>;
  onSelectAlternative: (index: number) => void;
  onToggleAlternatives: () => void;
  onOpenAlternativesWindow: () => void;
}

function RationTab({
  alternatives,
  alternativesCurrentIndex,
  alternativesExpanded,
  groupId,
  norms,
  onSelectAlternative,
  onToggleAlternatives,
  onOpenAlternativesWindow,
}: RationTabProps) {
  const { t } = useTranslation();
  const { animalCount, setAnimalCount } = useRationStore();

  const scrollToFeedLibrary = () => {
    const searchInput = document.querySelector('aside input[type="text"]') as HTMLInputElement | null;
    if (searchInput) {
      searchInput.focus();
      searchInput.scrollIntoView({ behavior: 'smooth' });
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-3 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
        <div>
          <label className="mb-1 block text-xs font-medium text-[--text-primary]">
            {t('newRation.animalCount')}
          </label>
          <input
            type="number"
            min={1}
            step={1}
            value={animalCount}
            onChange={(event) => setAnimalCount(Number.parseInt(event.target.value, 10) || 1)}
            className="w-28 rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs text-[--text-primary]"
          />
        </div>
        <p className="text-xs text-[--text-secondary]">
          {t('workspace.headCountHint')}
        </p>
      </div>

      <RationAppliedChangesPanel />
      <AlternativesPanel
        solutions={alternatives}
        currentIndex={alternativesCurrentIndex}
        groupId={groupId}
        norms={norms}
        expanded={alternativesExpanded}
        onSelect={onSelectAlternative}
        onToggleExpanded={onToggleAlternatives}
        onOpenWindow={onOpenAlternativesWindow}
      />
      <RationTable />
      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={scrollToFeedLibrary}>
          <Icon icon={Plus} size={14} className="mr-1.5" />
          {t('workspace.addFeed')}
        </Button>
      </div>
    </div>
  );
}

function NutrientsTab() {
  return <NutrientPanel />;
}

function EconomicsTab() {
  return <EconomicsPanel />;
}

function ReportTab() {
  const { t } = useTranslation();
  const { localItems, animalProperties, customNorms, activeNormPresetId } = useRationStore();
  const nutrients = localItems.length > 0 ? calculateLocalNutrients(localItems) : null;
  const { norms: currentNorms } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  const [includeNutrients, setIncludeNutrients] = useState(true);
  const [includeEconomics, setIncludeEconomics] = useState(true);
  const [includeNormsComparison, setIncludeNormsComparison] = useState(true);
  const [includeFeedDetails, setIncludeFeedDetails] = useState(false);
  const [title, setTitle] = useState('');
  const [notes, setNotes] = useState('');
  const [exportPreferences, setExportPreferences] = useState(() => loadExportPreferences());

  useEffect(() => {
    saveExportPreferences(exportPreferences);
  }, [exportPreferences]);

  const exportOpts = {
    includeNutrients,
    includeEconomics,
    includeNormsComparison,
    includeFeedDetails,
    title,
    notes,
    destinationDir: exportPreferences.destinationDir,
    fileName: exportPreferences.fileName,
    fontFamily: exportPreferences.fontFamily,
    appearance: exportPreferences.appearance,
  };

  const handleExportCsv = async () => {
    await exportCsv(localItems, nutrients, exportOpts);
  };

  const handleExportPdf = async () => {
    const norms = includeNormsComparison ? currentNorms : null;
    await exportPdf(localItems, nutrients, norms, exportOpts);
  };

  const handleExportExcel = async () => {
    const norms = includeNormsComparison ? currentNorms : null;
    await exportExcel(localItems, nutrients, norms, exportOpts);
  };

  return (
    <div className="space-y-4">
      <div className="bg-[--bg-surface] rounded-[--radius-md] p-4">
        <h3 className="text-sm font-medium text-[--text-primary] mb-3">{t('report.exportRationReport')}</h3>
        <p className="text-xs text-[--text-secondary] mb-4">{t('report.reportDescription')}</p>

        <div className="space-y-2 mb-4">
          <input
            type="text"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder={t('report.titlePlaceholder')}
            className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary] placeholder:text-[--text-disabled]"
          />
          <textarea
            value={notes}
            onChange={(event) => setNotes(event.target.value)}
            placeholder={t('report.notesPlaceholder')}
            rows={2}
            className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary] placeholder:text-[--text-disabled] resize-none"
          />
          <div className="grid gap-2 md:grid-cols-2">
            <div>
              <label className="mb-1 block text-[10px] text-[--text-secondary]">{t('report.destination')}</label>
              <input
                type="text"
                value={exportPreferences.destinationDir}
                onChange={(event) =>
                  setExportPreferences((previous) => ({
                    ...previous,
                    destinationDir: event.target.value,
                  }))
                }
                placeholder={t('report.destinationPlaceholder')}
                className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary] placeholder:text-[--text-disabled]"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] text-[--text-secondary]">{t('report.fileName')}</label>
              <input
                type="text"
                value={exportPreferences.fileName}
                onChange={(event) =>
                  setExportPreferences((previous) => ({
                    ...previous,
                    fileName: event.target.value,
                  }))
                }
                placeholder={t('report.fileNamePlaceholder')}
                className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary] placeholder:text-[--text-disabled]"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] text-[--text-secondary]">{t('report.fontFamily')}</label>
              <select
                value={exportPreferences.fontFamily}
                onChange={(event) =>
                  setExportPreferences((previous) => ({
                    ...previous,
                    fontFamily: event.target.value as 'sans' | 'serif' | 'mono',
                  }))
                }
                className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary]"
              >
                <option value="sans">{t('report.fontSans')}</option>
                <option value="serif">{t('report.fontSerif')}</option>
                <option value="mono">{t('report.fontMono')}</option>
              </select>
            </div>
            <div>
              <label className="mb-1 block text-[10px] text-[--text-secondary]">{t('report.appearance')}</label>
              <select
                value={exportPreferences.appearance}
                onChange={(event) =>
                  setExportPreferences((previous) => ({
                    ...previous,
                    appearance: event.target.value as 'standard' | 'compact' | 'presentation',
                  }))
                }
                className="w-full bg-[--bg-base] border border-[--border] rounded px-3 py-1.5 text-xs text-[--text-primary]"
              >
                <option value="standard">{t('report.appearanceStandard')}</option>
                <option value="compact">{t('report.appearanceCompact')}</option>
                <option value="presentation">{t('report.appearancePresentation')}</option>
              </select>
            </div>
          </div>
        </div>

        <div className="flex gap-2 flex-wrap">
          <Button variant="outline" size="sm" onClick={handleExportPdf} disabled={localItems.length === 0}>
            <Icon icon={FileText} size={14} className="mr-1.5" />
            {t('report.exportPdf')}
          </Button>
          <Button variant="outline" size="sm" onClick={handleExportExcel} disabled={localItems.length === 0}>
            {t('report.exportExcel')}
          </Button>
          <Button variant="outline" size="sm" onClick={handleExportCsv} disabled={localItems.length === 0}>
            {t('report.exportCsv')}
          </Button>
        </div>
      </div>

      <div className="bg-[--bg-surface] rounded-[--radius-md] p-4">
        <h3 className="text-sm font-medium text-[--text-primary] mb-3">{t('report.reportSettings')}</h3>
        <div className="space-y-2 text-xs text-[--text-secondary]">
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={includeNutrients} onChange={(event) => setIncludeNutrients(event.target.checked)} className="rounded" />
            {t('report.includeNutrientChart')}
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={includeEconomics} onChange={(event) => setIncludeEconomics(event.target.checked)} className="rounded" />
            {t('report.includeCostBreakdown')}
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={includeNormsComparison} onChange={(event) => setIncludeNormsComparison(event.target.checked)} className="rounded" />
            {t('report.includeNormsComparison')}
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={includeFeedDetails} onChange={(event) => setIncludeFeedDetails(event.target.checked)} className="rounded" />
            {t('report.includeFeedDetails')}
          </label>
        </div>
      </div>
    </div>
  );
}

function DashboardView() {
  const { t } = useTranslation();
  const { localItems, animalProperties, openWorkspaceProject } = useRationStore();
  const { feeds: feedCatalog } = useFeedCatalog();
  const presetsQuery = useQuery({
    queryKey: ['dashboardPresetCatalog'],
    queryFn: () => presetsApi.list(),
    staleTime: 5 * 60 * 1000,
  });
  const presetCategories = presetsQuery.data?.data.categories ?? [];
  const [selectedSpecies, setSelectedSpecies] = useState(animalProperties.species || 'cattle');
  const [selectedProductionType, setSelectedProductionType] = useState('');
  const [selectedPresetId, setSelectedPresetId] = useState<string | null>(null);
  const [isCreatingPreset, setIsCreatingPreset] = useState(false);

  const nutrients = localItems.length > 0 ? calculateLocalNutrients(localItems) : null;
  const totalCost = localItems.reduce((sum, item) => sum + item.amount_kg * ((item.feed.price_per_ton ?? 0) / 1000), 0);
  const availableProductionTypes = useMemo(
    () =>
      presetCategories
        .filter((category) => category.species === selectedSpecies)
        .map((category) => category.production_type),
    [presetCategories, selectedSpecies],
  );
  const selectedCategory = useMemo(
    () =>
      presetCategories.find(
        (category) =>
          category.species === selectedSpecies &&
          category.production_type === selectedProductionType,
      ) ?? null,
    [presetCategories, selectedProductionType, selectedSpecies],
  );
  const selectedPreset =
    selectedCategory?.subcategories.find((preset) => preset.id === selectedPresetId)
    ?? selectedCategory?.subcategories[0]
    ?? null;

  useEffect(() => {
    if (availableProductionTypes.length === 0) {
      if (selectedProductionType !== '') {
        setSelectedProductionType('');
      }
      return;
    }

    if (!availableProductionTypes.includes(selectedProductionType)) {
      setSelectedProductionType(availableProductionTypes[0]);
    }
  }, [availableProductionTypes, selectedProductionType]);

  useEffect(() => {
    if (!selectedCategory) {
      if (selectedPresetId !== null) {
        setSelectedPresetId(null);
      }
      return;
    }

    const presetIds = selectedCategory.subcategories.map((preset) => preset.id);
    if (!selectedPresetId || !presetIds.includes(selectedPresetId)) {
      setSelectedPresetId(selectedCategory.subcategories[0]?.id ?? null);
    }
  }, [selectedCategory, selectedPresetId]);

  const createWorkspaceProject = async (
    projectBase: Omit<RationProject, 'name'>,
    baseName: string,
  ) => {
    for (let attempt = 0; attempt < 25; attempt += 1) {
      const candidateName = attempt === 0 ? baseName : `${baseName} (${attempt + 1})`;
      const safeName = candidateName.replace(/[<>:"/\\|?*]/g, '_');
      const path = `${safeName}.felex.json`;
      const project: RationProject = {
        ...projectBase,
        name: candidateName,
      };

      try {
        await workspaceApi.createRation(path, project);
        openWorkspaceProject(path, project, feedCatalog);
        requestWorkspaceRefresh();
        toast.success(t('newRation.createAndOpen'));
        return;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes('already exists') && attempt < 24) {
          continue;
        }
        throw error;
      }
    }
  };

  const createProjectFromLegacyPreset = async (preset: typeof RATION_PRESETS[number]) => {
    const groupId = getGroupIdForPreset(preset);
    const presetProperties = getAnimalPropertiesForPreset(preset.id, groupId);
    const missingFeeds = preset.items.filter((item) => !findFeedByKnownName(feedCatalog, item.feedName));

    if (missingFeeds.length > 0) {
      toast.error(missingFeeds.slice(0, 3).map((item) => item.feedName).join(', '));
      return;
    }

    const projectBase: Omit<RationProject, 'name'> = {
      version: '1.0',
      animalGroupId: groupId,
      animalProperties: {
        species: presetProperties.species,
        productionType: presetProperties.productionType,
        breed: presetProperties.breed,
        sex: presetProperties.sex,
        ageMonths: presetProperties.ageToDays
          ? Math.round((presetProperties.ageToDays / 30) * 10) / 10
          : presetProperties.ageFromDays
            ? Math.round((presetProperties.ageFromDays / 30) * 10) / 10
            : undefined,
        weight: presetProperties.liveWeightKg,
        milkYieldKg: presetProperties.milkYieldKg,
        milkFatPercent: presetProperties.milkFatPct,
        dailyGainG: presetProperties.dailyGainG,
        eggProduction: presetProperties.eggProductionPerYear,
        litterSize: presetProperties.litterSize,
        stage: presetProperties.reproductiveStage,
      },
      animalCount: 1,
      items: preset.items.map((item) => {
        const feed = findFeedByKnownName(feedCatalog, item.feedName);
        if (!feed) {
          throw new Error(`Preset feed not found: ${item.feedName}`);
        }

        return {
          feedId: feed.id,
          feedName: feed.name_ru,
          amountKg: item.kgPerDay,
          isLocked: false,
        };
      }),
      normPresetId: NORM_PRESET_BY_RATION_PRESET[preset.id] ?? undefined,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    const baseName = `${preset.name_ru} ${new Date().toLocaleDateString('ru-RU')}`;

    try {
      await createWorkspaceProject(projectBase, baseName);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('common.error'));
    }
  };

  const buildAnimalPropertiesFromMatchedPreset = (
    preset: MatchedPresetSubcategory,
  ): AnimalProperties => {
    const base = getDefaultAnimalProperties(preset.animal_group_id);
    const ageDays = preset.params.age_days
      ?? (preset.params.age_weeks ? preset.params.age_weeks * 7 : undefined);
    const stage = preset.params.piglets
      ? 'lactation'
      : preset.params.days_pregnant
        ? 'gestation'
        : base.reproductiveStage;

    return {
      ...base,
      liveWeightKg: preset.params.live_weight_kg ?? base.liveWeightKg,
      milkYieldKg: preset.params.milk_yield_kg ?? base.milkYieldKg,
      dailyGainG: preset.params.daily_gain_g ?? base.dailyGainG,
      ageFromDays: ageDays ?? base.ageFromDays,
      ageToDays: ageDays ?? base.ageToDays,
      litterSize: preset.params.piglets ?? base.litterSize,
      reproductiveStage: stage,
      eggProductionPerYear: preset.params.production_pct
        ? Math.round((preset.params.production_pct / 100) * 330)
        : base.eggProductionPerYear,
    };
  };

  const starterItemsFromMatchedPreset = (preset: MatchedPresetSubcategory) => {
    const seen = new Set<number>();

    return preset.recommendations.flatMap((recommendation) => {
      const match = recommendation.matches[0];
      if (!match || seen.has(match.feed.id)) {
        return [];
      }

      seen.add(match.feed.id);
      return [{
        feedId: match.feed.id,
        feedName: capitalizeFirst(match.feed.name_ru) ?? match.feed.name_ru,
        amountKg: 0,
        isLocked: false,
      }];
    });
  };

  const createProjectFromMatchedPreset = async (
    preset: MatchedPresetSubcategory,
    mode: 'quick' | 'customize',
  ) => {
    setIsCreatingPreset(true);

    try {
      if (mode === 'quick' && preset.legacy_preset_id) {
        const legacyPreset = RATION_PRESETS.find((candidate) => candidate.id === preset.legacy_preset_id);
        if (legacyPreset) {
          await createProjectFromLegacyPreset(legacyPreset);
          return;
        }
      }

      const presetProperties = buildAnimalPropertiesFromMatchedPreset(preset);
      const starterItems = mode === 'quick' ? starterItemsFromMatchedPreset(preset) : [];
      if (mode === 'quick' && starterItems.length === 0) {
        toast.error(t('dashboard.noRecommendationMatches'));
        return;
      }

      const projectBase: Omit<RationProject, 'name'> = {
        version: '1.0',
        animalGroupId: preset.animal_group_id,
        animalProperties: {
          species: presetProperties.species,
          productionType: presetProperties.productionType,
          breed: presetProperties.breed,
          sex: presetProperties.sex,
          ageMonths: presetProperties.ageToDays
            ? Math.round((presetProperties.ageToDays / 30) * 10) / 10
            : presetProperties.ageFromDays
              ? Math.round((presetProperties.ageFromDays / 30) * 10) / 10
              : undefined,
          weight: presetProperties.liveWeightKg,
          milkYieldKg: presetProperties.milkYieldKg,
          milkFatPercent: presetProperties.milkFatPct,
          dailyGainG: presetProperties.dailyGainG,
          eggProduction: presetProperties.eggProductionPerYear,
          litterSize: presetProperties.litterSize,
          stage: presetProperties.reproductiveStage,
        },
        animalCount: 1,
        items: starterItems,
        normPresetId: preset.norm_preset_id ?? undefined,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const baseName = `${preset.name_ru} ${new Date().toLocaleDateString('ru-RU')}`;
      await createWorkspaceProject(projectBase, baseName);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('common.error'));
    } finally {
      setIsCreatingPreset(false);
    }
  };

  const getGroupDisplayName = () => {
    const typeNames: Record<string, string> = {
      dairy: t('nav.dairyCows'),
      beef: t('nav.beefCattle'),
      fattening: t('nav.finishers'),
      breeding: t('nav.sows'),
      broiler: t('nav.broilers'),
      layer: t('nav.layers'),
    };

    return typeNames[animalProperties.productionType] || t('nav.dairyCows');
  };

  return (
    <main className="flex-1 min-w-0 flex flex-col overflow-hidden bg-[--bg-base]">
      <header className="px-4 py-3 border-b border-[--border]">
        <div className="flex items-center gap-3">
          <Icon icon={Home} size={20} className="text-[--accent]" />
          <h1 className="text-sm font-medium text-[--text-primary]">{t('dashboard.title')}</h1>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-4 space-y-4">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
          <MetricCard value={String(localItems.length)} label={t('dashboard.feedsInRation')} accent />
          <MetricCard value={nutrients ? formatNumber(nutrients.total_weight_kg, 1) : '—'} label={`${t('dashboard.totalWeight')}, ${t('units.kg')}`} />
          <MetricCard value={nutrients ? formatNumber(nutrients.energy_eke, 1) : '—'} label={t('dashboard.energyEke')} />
          <MetricCard value={totalCost > 0 ? formatCurrency(totalCost) : '—'} label={t('dashboard.dailyCost')} accent />
        </div>

        <div className="bg-[--bg-surface] rounded-[--radius-md] p-4">
          <h3 className="text-xs font-medium text-[--text-secondary] mb-3 uppercase tracking-wide">{t('dashboard.currentRation')}</h3>
          <p className="text-sm text-[--text-primary] mb-1">{getGroupDisplayName()}</p>
          <p className="text-xs text-[--text-secondary]">
            {animalProperties.breed} | {animalProperties.liveWeightKg} {t('units.kg')}
            {animalProperties.milkYieldKg ? ` | ${animalProperties.milkYieldKg} ${t('animal.kgDay')}` : ''}
            {animalProperties.dailyGainG ? ` | ${animalProperties.dailyGainG} ${t('norms.gDay')}` : ''}
          </p>
        </div>

        <PresetSelector
          categories={presetCategories}
          selectedSpecies={selectedSpecies}
          selectedProductionType={selectedProductionType}
          selectedPresetId={selectedPreset?.id ?? null}
          onSpeciesChange={setSelectedSpecies}
          onProductionTypeChange={setSelectedProductionType}
          onPresetChange={setSelectedPresetId}
          loading={presetsQuery.isLoading}
        />

        <FeedRecommendations
          preset={selectedPreset}
          species={selectedSpecies}
          onQuickStart={() => {
            if (selectedPreset) {
              void createProjectFromMatchedPreset(selectedPreset, 'quick');
            }
          }}
          onCustomize={() => {
            if (selectedPreset) {
              void createProjectFromMatchedPreset(selectedPreset, 'customize');
            }
          }}
          busy={isCreatingPreset}
        />

        <p className="text-xs text-[--text-disabled] text-center">{t('dashboard.selectGroupHint')}</p>
      </div>
    </main>
  );
}

function MetricCard({ value, label, accent = false }: { value: string; label: string; accent?: boolean }) {
  return (
    <div className="bg-[--bg-surface] rounded-[--radius-md] p-4 text-center">
      <div className={cn('text-2xl font-bold', accent ? 'text-[--accent]' : 'text-[--text-primary]')}>{value}</div>
      <div className="text-[10px] text-[--text-secondary] mt-1">{label}</div>
    </div>
  );
}

function NormsView() {
  const { t, i18n } = useTranslation();
  const { animalProperties, activeNormPresetId } = useRationStore();
  const currentGroupId = resolveAnimalGroupId(animalProperties);
  const {
    baseNorms: resolvedCurrentNorms,
    methodology: currentMethodology,
    normSource,
  } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
  );

  const [selectedGroup, setSelectedGroup] = useState(currentGroupId);
  const [selectedPresetId, setSelectedPresetId] = useState<string | null>(activeNormPresetId);
  const [showMethodology, setShowMethodology] = useState(false);
  const [editedNorms, setEditedNorms] = useState<Record<string, NormRange>>({});
  const [isEditing, setIsEditing] = useState(false);

  useEffect(() => {
    setSelectedGroup(currentGroupId);
    setSelectedPresetId(activeNormPresetId);
    setEditedNorms({});
    setIsEditing(false);
  }, [activeNormPresetId, currentGroupId]);

  const groups = [
    { id: 'cattle_dairy', label: t('norms.dairyCattle') },
    { id: 'cattle_beef', label: t('norms.beefCattle') },
    { id: 'swine_finisher', label: t('norms.swineFinisher') },
    { id: 'swine_sow', label: t('norms.swineBreeding') },
    { id: 'poultry_broiler', label: t('norms.broiler') },
    { id: 'poultry_layer', label: t('norms.layer') },
  ];

  const presets = getPresetsForGroup(selectedGroup);
  const selectedPreset = presets.find((preset) => preset.id === selectedPresetId) ?? null;
  const showCurrentMethodology =
    selectedGroup === currentGroupId && selectedPresetId === activeNormPresetId;
  const defaultNorms = useMemo(() => {
    if (selectedPreset) {
      return selectedPreset.norms;
    }

    if (selectedGroup === currentGroupId) {
      return resolvedCurrentNorms;
    }

    return getBaseNormsForGroupId(selectedGroup);
  }, [currentGroupId, resolvedCurrentNorms, selectedGroup, selectedPreset]);

  const normsData = isEditing ? { ...defaultNorms, ...editedNorms } : defaultNorms;
  const normKeys = useMemo(
    () => getOrderedNutrientKeys(Object.keys(normsData)),
    [normsData],
  );

  const handleGroupChange = (groupId: string) => {
    setSelectedGroup(groupId);
    setSelectedPresetId(groupId === currentGroupId ? activeNormPresetId : null);
    setEditedNorms({});
    setIsEditing(false);
  };

  const handlePresetChange = (presetId: string) => {
    setSelectedPresetId(presetId);
    setEditedNorms({});
    setIsEditing(false);
  };

  const handleNormEdit = (key: string, field: 'min' | 'max' | 'target', value: string) => {
    const parsed = value === '' ? undefined : Number.parseFloat(value);
    setEditedNorms((previous) => ({
      ...previous,
      [key]: {
        ...(defaultNorms[key] ?? {}),
        ...(previous[key] ?? {}),
        [field]: Number.isFinite(parsed as number) ? parsed : undefined,
      },
    }));
  };

  const handleResetNorms = () => {
    setEditedNorms({});
    setIsEditing(false);
  };

  return (
    <main className="flex-1 min-w-0 flex flex-col overflow-hidden bg-[--bg-base]">
      <header className="px-4 py-3 border-b border-[--border]">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <Icon icon={BookOpen} size={20} className="text-[--accent]" />
            <div className="min-w-0">
              <h1 className="text-sm font-medium text-[--text-primary]">{t('norms.title')}</h1>
              <p className="text-[10px] text-[--text-secondary]">{t('norms.description')}</p>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {isEditing ? (
              <Button variant="outline" size="sm" onClick={handleResetNorms}>
                <Icon icon={RotateCcw} size={14} className="mr-1" />
                {t('norms.resetDefaults')}
              </Button>
            ) : null}
            <Button variant={isEditing ? 'default' : 'outline'} size="sm" onClick={() => setIsEditing((value) => !value)}>
              {isEditing ? t('norms.doneEditing') : t('norms.editNorms')}
            </Button>
          </div>
        </div>
      </header>

      <div className="flex items-center gap-1 px-4 py-2 border-b border-[--border] flex-wrap">
        {groups.map((group) => (
          <button
            key={group.id}
            onClick={() => handleGroupChange(group.id)}
            className={cn(
              'px-3 py-1.5 text-xs rounded-[--radius-sm] transition-colors',
              selectedGroup === group.id
                ? 'bg-[--bg-active] text-[--accent]'
                : 'text-[--text-secondary] hover:bg-[--bg-hover] hover:text-[--text-primary]'
            )}
          >
            {group.label}
          </button>
        ))}
      </div>

      {presets.length > 0 ? (
        <div className="flex items-center gap-1 px-4 py-2 border-b border-[--border] bg-[--bg-surface] flex-wrap">
          <span className="text-[10px] text-[--text-disabled] mr-2">{t('norms.productionLevel')}:</span>
          {presets.map((preset) => (
            <button
              key={preset.id}
              onClick={() => handlePresetChange(preset.id)}
              className={cn(
                'px-2 py-1 text-[10px] rounded-[--radius-sm] transition-colors',
                selectedPresetId === preset.id
                  ? 'bg-[--accent] text-white'
                  : 'bg-[--bg-hover] text-[--text-secondary] hover:text-[--text-primary]'
              )}
            >
              {i18n.language.startsWith('en') ? preset.label_en : preset.label_ru}
            </button>
          ))}
        </div>
      ) : null}

      <div className="flex-1 overflow-auto p-4 space-y-4">
        <div className="bg-[--bg-surface] rounded-[--radius-md] border border-[--border]">
          <button
            onClick={() => setShowMethodology((value) => !value)}
            className="w-full flex items-center justify-between px-4 py-3 text-xs font-medium text-[--text-primary]"
          >
            <span>{t('norms.methodology')}</span>
            <Icon icon={showMethodology ? ChevronUp : ChevronDown} size={14} className="text-[--text-disabled]" />
          </button>
          {showMethodology ? (
            <div className="border-t border-[--border] p-4">
              <NormMethodologyCard
                methodology={showCurrentMethodology ? currentMethodology : null}
                normSource={showCurrentMethodology ? normSource : 'backend'}
                showFormulaMarkdown
                showCitationMarkdown
                emptyStateMessage={showCurrentMethodology ? null : t('norms.methodologyActiveOnly')}
              />
            </div>
          ) : null}
        </div>

        {selectedPreset ? (
          <div className="bg-[--bg-surface] rounded-[--radius-md] p-3 border border-[--border]">
            <div className="flex items-center gap-4 text-xs flex-wrap">
              <span className="font-medium text-[--text-primary]">
                {i18n.language.startsWith('en') ? selectedPreset.label_en : selectedPreset.label_ru}
              </span>
              {selectedPreset.params.weight ? <span className="text-[--text-secondary]">{t('norms.weight')}: {selectedPreset.params.weight} {t('units.kg')}</span> : null}
              {selectedPreset.params.milkYield ? <span className="text-[--text-secondary]">{t('norms.milkYield')}: {selectedPreset.params.milkYield} {t('animal.kgDay')}</span> : null}
              {selectedPreset.params.dailyGain ? <span className="text-[--text-secondary]">{t('norms.dailyGain')}: {selectedPreset.params.dailyGain} {t('norms.gDay')}</span> : null}
              {selectedPreset.params.age ? <span className="text-[--text-secondary]">{t('norms.age')}: {selectedPreset.params.age}</span> : null}
            </div>
          </div>
        ) : null}

        <table className="w-full text-xs border border-[--border] rounded-[--radius-md] overflow-hidden">
          <thead className="bg-[--bg-surface]">
            <tr>
              <th className="text-left px-4 py-2 font-medium text-[--text-secondary]">{t('norms.nutrient')}</th>
              <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-24">{t('norms.min')}</th>
              <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-24">{t('norms.target')}</th>
              <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-24">{t('norms.max')}</th>
              <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-16">{t('norms.unit')}</th>
            </tr>
          </thead>
          <tbody>
            {normKeys.map((key) => {
              const norm = normsData[key];
              if (!norm && !isEditing) {
                return null;
              }

              const edited = editedNorms[key];
              const displayNorm = norm ?? {};

              return (
                <tr key={key} className="border-t border-[--border] hover:bg-[--bg-hover]">
                  <td className="px-4 py-2 text-[--text-primary]">
                    {getNutrientLabel(key, i18n.language.startsWith('en') ? 'en' : 'ru')}
                    {edited ? <span className="ml-1 text-[--accent]">*</span> : null}
                  </td>
                  {isEditing ? (
                    <>
                      <td className="px-1 py-1">
                        <input
                          type="number"
                          step="any"
                          className="w-full text-right bg-[--bg-base] border border-[--border] rounded px-2 py-1 text-xs"
                          value={displayNorm.min ?? ''}
                          onChange={(event) => handleNormEdit(key, 'min', event.target.value)}
                        />
                      </td>
                      <td className="px-1 py-1">
                        <input
                          type="number"
                          step="any"
                          className="w-full text-right bg-[--bg-base] border border-[--border] rounded px-2 py-1 text-xs font-medium"
                          value={displayNorm.target ?? ''}
                          onChange={(event) => handleNormEdit(key, 'target', event.target.value)}
                        />
                      </td>
                      <td className="px-1 py-1">
                        <input
                          type="number"
                          step="any"
                          className="w-full text-right bg-[--bg-base] border border-[--border] rounded px-2 py-1 text-xs"
                          value={displayNorm.max ?? ''}
                          onChange={(event) => handleNormEdit(key, 'max', event.target.value)}
                        />
                      </td>
                    </>
                  ) : (
                    <>
                      <td className="px-3 py-2 text-right text-[--text-secondary]">{displayNorm.min ?? '—'}</td>
                      <td className="px-3 py-2 text-right font-medium text-[--text-primary]">{displayNorm.target ?? '—'}</td>
                      <td className="px-3 py-2 text-right text-[--text-secondary]">{displayNorm.max ?? '—'}</td>
                    </>
                  )}
                  <td className="px-3 py-2 text-right text-[--text-disabled]">
                    {getNutrientUnit(key, i18n.language.startsWith('en') ? 'en' : 'ru')}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>

        <p className="text-[10px] text-[--text-disabled] mt-3 text-center">{t('norms.source')}</p>
      </div>
    </main>
  );
}
