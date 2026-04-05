import { create } from 'zustand';
import type { DietSolution, NutrientSummary } from '@/types/ration';
import type { Feed } from '@/types/feed';
import type { NormRange } from '@/types/nutrient';
import type { RationProject } from '@/types/ration-project';
import type { AlternativeRationSolution, OptimizationResult } from '@/types/optimization';
import { findFeedInCatalogByKnownName } from '@/lib/feed-display';

export interface AnimalProperties {
  species: string;
  productionType: string;
  breed: string;
  sex: 'male' | 'female' | 'mixed';
  liveWeightKg: number;
  ageFromDays?: number;
  ageToDays?: number;
  milkYieldKg?: number;
  milkFatPct?: number;
  dailyGainG?: number;
  eggProductionPerYear?: number;
  litterSize?: number;
  reproductiveStage?: 'gestation' | 'lactation';
}

interface LocalRationItem {
  id: string;
  feed: Feed;
  amount_kg: number;
  is_locked: boolean;
}

interface LoadPresetOptions {
  groupId?: string;
  animalProperties?: AnimalProperties;
  activeView?: string;
  normPresetId?: string | null;
}

export interface OptimizationFeedbackState {
  solution: DietSolution;
  isStale: boolean;
  updatedAt: string;
}

interface RationStore {
  currentRationId: number | null;
  currentProjectPath: string | null;
  currentProjectName: string | null;
  currentProjectCreatedAt: string | null;
  animalGroupId: string | null;
  animalProperties: AnimalProperties;
  animalCount: number;
  activeView: string;
  localItems: LocalRationItem[];
  nutrients: NutrientSummary | null;
  optimizationFeedback: OptimizationFeedbackState | null;
  customNorms: Record<string, NormRange> | null;
  activeNormPresetId: string | null;
  // Alternatives state
  alternatives: AlternativeRationSolution[];
  alternativesCurrentIndex: number;
  alternativesShowModal: boolean;
  alternativesPanelExpanded: boolean;
  alternativesPendingApply: boolean;
  setCurrentRation: (id: number | null, groupId?: string | null) => void;
  setCurrentProject: (path: string | null, name?: string | null, createdAt?: string | null) => void;
  setLocalItems: (items: LocalRationItem[]) => void;
  addFeed: (feed: Feed, amount?: number) => void;
  removeFeed: (localId: string) => void;
  updateAmount: (localId: string, amount: number) => void;
  toggleLock: (localId: string) => void;
  clearRation: () => void;
  setAnimalCount: (count: number) => void;
  setAnimalProperties: (props: AnimalProperties) => void;
  setActiveView: (view: string) => void;
  setNutrients: (nutrients: NutrientSummary | null) => void;
  setOptimizationFeedback: (solution: DietSolution | null) => void;
  setCustomNorm: (key: string, range: NormRange) => void;
  resetNorms: () => void;
  setActiveNormPreset: (presetId: string | null) => void;
  openWorkspaceProject: (path: string, project: RationProject, feedCatalog: Feed[]) => void;
  loadPreset: (feeds: Feed[], items: Array<{ feedName: string; kgPerDay: number }>, options?: LoadPresetOptions) => void;
  // Alternatives actions
  setAlternatives: (
    result: OptimizationResult,
    options?: { pendingApply?: boolean }
  ) => void;
  selectAlternative: (index: number, feedCatalog: Feed[]) => void;
  toggleAlternativesModal: () => void;
  toggleAlternativesPanel: () => void;
  clearAlternatives: () => void;
  // Farm bucket
  farmBucket: Set<number>;
  farmBucketActive: boolean;
  toggleFarmBucketFeed: (feedId: number) => void;
  addToFarmBucket: (feedIds: number[]) => void;
  removeFromFarmBucket: (feedIds: number[]) => void;
  setFarmBucketActive: (active: boolean) => void;
  clearFarmBucket: () => void;
}

const ANIMAL_GROUP_IDS = [
  'cattle_dairy',
  'cattle_beef',
  'swine_finisher',
  'swine_sow',
  'poultry_broiler',
  'poultry_layer',
] as const;

let itemCounter = 0;

function normalizeProductionType(species: string, productionType: string): string {
  if (species !== 'swine') {
    return productionType;
  }

  if (productionType === 'finisher') {
    return 'fattening';
  }
  if (productionType === 'sow') {
    return 'breeding';
  }

  return productionType;
}

export function resolveAnimalGroupId(props: Pick<AnimalProperties, 'species' | 'productionType'>): string {
  const normalizedProductionType = normalizeProductionType(props.species, props.productionType);
  const key = `${props.species}_${normalizedProductionType}`;
  const mapping: Record<string, string> = {
    cattle_dairy: 'cattle_dairy',
    cattle_beef: 'cattle_beef',
    swine_finisher: 'swine_finisher',
    swine_sow: 'swine_sow',
    swine_fattening: 'swine_finisher',
    swine_breeding: 'swine_sow',
    poultry_broiler: 'poultry_broiler',
    poultry_layer: 'poultry_layer',
  };

  return mapping[key] ?? 'cattle_dairy';
}

export function isRationWorkspaceView(view: string): boolean {
  return (ANIMAL_GROUP_IDS as readonly string[]).includes(view);
}

export function getGroupIdForPreset(preset: { species: string; productionType: string }): string {
  if (preset.species === 'cattle') {
    return preset.productionType === 'beef' ? 'cattle_beef' : 'cattle_dairy';
  }
  if (preset.species === 'swine') {
    const normalizedProductionType = normalizeProductionType(preset.species, preset.productionType);
    return normalizedProductionType === 'breeding' ? 'swine_sow' : 'swine_finisher';
  }
  return preset.productionType === 'layer' ? 'poultry_layer' : 'poultry_broiler';
}

export function getDefaultAnimalProperties(groupId: string): AnimalProperties {
  const defaults: Record<string, AnimalProperties> = {
    cattle_dairy: {
      species: 'cattle',
      productionType: 'dairy',
      breed: 'Голштинская',
      sex: 'female',
      liveWeightKg: 620,
      milkYieldKg: 35,
      milkFatPct: 3.7,
      ageFromDays: 60,
      ageToDays: 120,
    },
    cattle_beef: {
      species: 'cattle',
      productionType: 'beef',
      breed: 'Абердин-ангусская',
      sex: 'male',
      liveWeightKg: 450,
      dailyGainG: 1000,
      ageFromDays: 360,
      ageToDays: 540,
    },
    swine_finisher: {
      species: 'swine',
      productionType: 'fattening',
      breed: 'Крупная белая',
      sex: 'mixed',
      liveWeightKg: 80,
      dailyGainG: 900,
      ageFromDays: 120,
      ageToDays: 170,
    },
    swine_sow: {
      species: 'swine',
      productionType: 'breeding',
      breed: 'Ландрас',
      sex: 'female',
      liveWeightKg: 210,
      litterSize: 11,
      reproductiveStage: 'lactation',
    },
    poultry_broiler: {
      species: 'poultry',
      productionType: 'broiler',
      breed: 'Кобб 500',
      sex: 'mixed',
      liveWeightKg: 2.4,
      dailyGainG: 65,
      ageFromDays: 25,
      ageToDays: 35,
    },
    poultry_layer: {
      species: 'poultry',
      productionType: 'layer',
      breed: 'Ломанн Браун',
      sex: 'female',
      liveWeightKg: 1.8,
      eggProductionPerYear: 320,
      ageFromDays: 160,
      ageToDays: 320,
    },
  };

  return defaults[groupId] ?? defaults.cattle_dairy;
}

export function getAnimalPropertiesForPreset(presetId: string, groupId: string): AnimalProperties {
  const base = getDefaultAnimalProperties(groupId);

  const overrides: Record<string, Partial<AnimalProperties>> = {
    dairy_25: { liveWeightKg: 580, milkYieldKg: 25, milkFatPct: 3.8, breed: 'Чёрно-пёстрая' },
    dairy_35: { liveWeightKg: 640, milkYieldKg: 35, milkFatPct: 3.7, breed: 'Голштинская' },
    beef_400: { liveWeightKg: 400, dailyGainG: 1100, sex: 'male', breed: 'Абердин-ангусская' },
    beef_550: { liveWeightKg: 550, dailyGainG: 900, sex: 'male', breed: 'Шароле' },
    swine_starter: { liveWeightKg: 25, dailyGainG: 600, ageFromDays: 50, ageToDays: 70, sex: 'mixed', breed: 'Крупная белая' },
    swine_finisher: { liveWeightKg: 80, dailyGainG: 900, ageFromDays: 120, ageToDays: 170, sex: 'mixed', breed: 'Дюрок' },
    swine_sow_gestation: { liveWeightKg: 200, reproductiveStage: 'gestation', litterSize: 0, sex: 'female', breed: 'Ландрас' },
    swine_sow_lactation: { liveWeightKg: 210, reproductiveStage: 'lactation', litterSize: 11, sex: 'female', breed: 'Ландрас' },
    broiler_starter: { liveWeightKg: 0.25, dailyGainG: 30, ageFromDays: 0, ageToDays: 10, sex: 'mixed', breed: 'Кобб 500' },
    broiler_finisher: { liveWeightKg: 2.4, dailyGainG: 70, ageFromDays: 25, ageToDays: 42, sex: 'mixed', breed: 'Росс 308' },
    layer_phase1: { liveWeightKg: 1.8, eggProductionPerYear: 320, ageFromDays: 140, ageToDays: 315, sex: 'female', breed: 'Ломанн Браун' },
    layer_phase2: { liveWeightKg: 1.95, eggProductionPerYear: 285, ageFromDays: 315, ageToDays: 500, sex: 'female', breed: 'Хайсекс Браун' },
  };

  return {
    ...base,
    ...(overrides[presetId] ?? {}),
  };
}

function resolveFeedForProjectItem(
  feedCatalog: Feed[],
  item: RationProject['items'][number]
): Feed {
  const byId = feedCatalog.find(
    (feed) =>
      feed.id === item.feedId &&
      findFeedInCatalogByKnownName([feed], item.feedName),
  );
  if (byId) {
    return byId;
  }

  const byName = findFeedInCatalogByKnownName(feedCatalog, item.feedName)
    ?? (() => {
      const normalizedName = item.feedName.trim().toLowerCase();
      return feedCatalog.find((feed) =>
        feed.name_ru.toLowerCase() === normalizedName ||
        feed.name_en?.toLowerCase() === normalizedName
      );
    })();
  if (byName) {
    return byName;
  }

  return {
    id: item.feedId,
    name_ru: item.feedName,
    category: 'other',
    dry_matter: 86,
  };
}

function recalculate(items: LocalRationItem[]): NutrientSummary | null {
  return items.length > 0 ? calculateLocalNutrients(items) : null;
}

function rationSignature(items: LocalRationItem[]): string {
  return items
    .map((item) => `${item.id}:${item.feed.id}:${item.amount_kg.toFixed(4)}:${item.is_locked ? 1 : 0}:${item.feed.price_per_ton ?? 'na'}`)
    .sort()
    .join('|');
}

function markFeedbackStale(
  feedback: OptimizationFeedbackState | null
): OptimizationFeedbackState | null {
  if (!feedback || feedback.isStale) {
    return feedback;
  }

  return {
    ...feedback,
    isStale: true,
  };
}

const SAMPLE_FEEDS: Feed[] = [
  {
    id: 1,
    name_ru: 'Силос кукурузный',
    name_en: 'Corn silage',
    category: 'roughage',
    dry_matter: 32,
    energy_oe_cattle: 10.5,
    crude_protein: 26,
    crude_fiber: 75,
    calcium: 1.2,
    phosphorus: 0.8,
    price_per_ton: 900,
  },
  {
    id: 2,
    name_ru: 'Сено люцерны',
    name_en: 'Alfalfa hay',
    category: 'roughage',
    dry_matter: 88,
    energy_oe_cattle: 9.2,
    crude_protein: 180,
    crude_fiber: 280,
    calcium: 14,
    phosphorus: 2.5,
    lysine: 8.5,
    methionine_cystine: 2.8,
    price_per_ton: 8000,
  },
  {
    id: 3,
    name_ru: 'Ячмень дробленый',
    name_en: 'Barley meal',
    category: 'concentrate',
    dry_matter: 87,
    energy_oe_cattle: 12.8,
    crude_protein: 115,
    crude_fiber: 52,
    calcium: 0.5,
    phosphorus: 3.5,
    lysine: 4.2,
    methionine_cystine: 1.8,
    price_per_ton: 12000,
  },
  {
    id: 4,
    name_ru: 'Шрот подсолнечный',
    name_en: 'Sunflower meal',
    category: 'protein',
    dry_matter: 90,
    energy_oe_cattle: 10.8,
    crude_protein: 380,
    crude_fiber: 140,
    calcium: 3.8,
    phosphorus: 11,
    lysine: 13.5,
    methionine_cystine: 7.2,
    price_per_ton: 22000,
  },
];

const initialItems: LocalRationItem[] = [
  { id: 'local-1', feed: SAMPLE_FEEDS[0], amount_kg: 20, is_locked: false },
  { id: 'local-2', feed: SAMPLE_FEEDS[1], amount_kg: 4, is_locked: false },
  { id: 'local-3', feed: SAMPLE_FEEDS[2], amount_kg: 5, is_locked: true },
  { id: 'local-4', feed: SAMPLE_FEEDS[3], amount_kg: 2, is_locked: false },
];

itemCounter = 4;

const FARM_BUCKET_KEY = 'felex:farm-bucket';

function loadFarmBucket(): Set<number> {
  try {
    const raw = localStorage.getItem(FARM_BUCKET_KEY);
    if (raw) {
      const ids = JSON.parse(raw) as number[];
      return new Set(ids);
    }
  } catch { /* ignore */ }
  return new Set();
}

function saveFarmBucket(bucket: Set<number>) {
  try {
    localStorage.setItem(FARM_BUCKET_KEY, JSON.stringify([...bucket]));
  } catch { /* ignore */ }
}

export const useRationStore = create<RationStore>((set, get) => ({
  currentRationId: null,
  currentProjectPath: null,
  currentProjectName: null,
  currentProjectCreatedAt: null,
  animalGroupId: 'cattle_dairy',
  animalProperties: getDefaultAnimalProperties('cattle_dairy'),
  animalCount: 1,
  activeView: 'dashboard',
  localItems: initialItems,
  nutrients: recalculate(initialItems),
  optimizationFeedback: null,
  customNorms: null,
  activeNormPresetId: null,
  alternatives: [],
  alternativesCurrentIndex: 0,
  alternativesShowModal: false,
  alternativesPanelExpanded: false,
  alternativesPendingApply: false,
  farmBucket: loadFarmBucket(),
  farmBucketActive: false,

  setCurrentRation: (id, groupId) =>
    set({
      currentRationId: id,
      animalGroupId: groupId ?? get().animalGroupId,
    }),

  setCurrentProject: (path, name, createdAt) =>
    set((state) => ({
      currentProjectPath: path,
      currentProjectName: name ?? (path ? state.currentProjectName : null),
      currentProjectCreatedAt: createdAt ?? (path ? state.currentProjectCreatedAt : null),
    })),

  setLocalItems: (items) =>
    set((state) => ({
      localItems: items,
      nutrients: recalculate(items),
      optimizationFeedback: rationSignature(state.localItems) === rationSignature(items)
        ? state.optimizationFeedback
        : markFeedbackStale(state.optimizationFeedback),
    })),

  addFeed: (feed, amount = 1.0) => {
    const localId = `local-${++itemCounter}`;
    set((state) => {
      const localItems = [
        ...state.localItems,
        {
          id: localId,
          feed,
          amount_kg: amount,
          is_locked: false,
        },
      ];

      return {
        localItems,
        nutrients: recalculate(localItems),
        optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
        alternatives: [],
        alternativesCurrentIndex: 0,
        alternativesShowModal: false,
        alternativesPanelExpanded: false,
        alternativesPendingApply: false,
      };
    });
  },

  removeFeed: (localId) =>
    set((state) => {
      const localItems = state.localItems.filter((item) => item.id !== localId);
      return {
        localItems,
        nutrients: recalculate(localItems),
        optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
        alternatives: [],
        alternativesCurrentIndex: 0,
        alternativesShowModal: false,
        alternativesPanelExpanded: false,
        alternativesPendingApply: false,
      };
    }),

  updateAmount: (localId, amount) =>
    set((state) => {
      const localItems = state.localItems.map((item) =>
        item.id === localId ? { ...item, amount_kg: Math.max(0, amount) } : item,
      );
      return {
        localItems,
        nutrients: recalculate(localItems),
        optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
        alternatives: [],
        alternativesCurrentIndex: 0,
        alternativesShowModal: false,
        alternativesPanelExpanded: false,
        alternativesPendingApply: false,
      };
    }),

  toggleLock: (localId) =>
    set((state) => ({
      localItems: state.localItems.map((item) =>
        item.id === localId ? { ...item, is_locked: !item.is_locked } : item,
      ),
      optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
    })),

  clearRation: () =>
    set({
      localItems: [],
      nutrients: null,
      optimizationFeedback: null,
      currentRationId: null,
      currentProjectPath: null,
      currentProjectName: null,
      currentProjectCreatedAt: null,
      activeNormPresetId: null,
      customNorms: null,
      animalCount: 1,
      activeView: 'dashboard',
      alternatives: [],
      alternativesCurrentIndex: 0,
      alternativesShowModal: false,
      alternativesPanelExpanded: false,
      alternativesPendingApply: false,
    }),

  setAnimalCount: (count) =>
    set({
      animalCount: Math.max(1, Math.round(count) || 1),
    }),

  setAnimalProperties: (props) =>
    set((state) => {
      const normalizedProps = {
        ...props,
        productionType: normalizeProductionType(props.species, props.productionType),
      };
      const groupId = resolveAnimalGroupId(normalizedProps);
      return {
        animalProperties: normalizedProps,
        animalGroupId: groupId,
        activeView: isRationWorkspaceView(state.activeView) ? groupId : state.activeView,
        optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
        activeNormPresetId: null,
        customNorms: null,
      };
    }),

  setActiveView: (view) =>
    set((state) => ({
      activeView: view,
      animalGroupId: isRationWorkspaceView(view) ? view : state.animalGroupId,
    })),

  setNutrients: (nutrients) => set({ nutrients }),

  setOptimizationFeedback: (solution) =>
    set({
      optimizationFeedback: solution
        ? {
          solution,
          isStale: false,
          updatedAt: new Date().toISOString(),
        }
        : null,
    }),

  setCustomNorm: (key, range) =>
    set((state) => ({
      customNorms: {
        ...(state.customNorms ?? {}),
        [key]: range,
      },
      optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
    })),

  resetNorms: () =>
    set((state) => ({
      customNorms: null,
      optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
    })),

  setActiveNormPreset: (presetId) =>
    set((state) => ({
      activeNormPresetId: presetId,
      optimizationFeedback: markFeedbackStale(state.optimizationFeedback),
    })),

  openWorkspaceProject: (path, project, feedCatalog) => {
    const groupId = project.animalGroupId || 'cattle_dairy';
    const defaultProps = getDefaultAnimalProperties(groupId);
    const species = project.animalProperties?.species || defaultProps.species;
    const productionType = normalizeProductionType(
      species,
      project.animalProperties?.productionType || defaultProps.productionType
    );
    const animalProperties: AnimalProperties = {
      ...defaultProps,
      species,
      productionType,
      breed: project.animalProperties?.breed || defaultProps.breed,
      sex: (project.animalProperties?.sex as AnimalProperties['sex']) || defaultProps.sex,
      liveWeightKg: project.animalProperties?.weight ?? defaultProps.liveWeightKg,
      milkYieldKg: project.animalProperties?.milkYieldKg ?? defaultProps.milkYieldKg,
      milkFatPct: project.animalProperties?.milkFatPercent ?? defaultProps.milkFatPct,
      dailyGainG: project.animalProperties?.dailyGainG ?? defaultProps.dailyGainG,
      eggProductionPerYear: project.animalProperties?.eggProduction ?? defaultProps.eggProductionPerYear,
      litterSize: project.animalProperties?.litterSize ?? defaultProps.litterSize,
      reproductiveStage: (project.animalProperties?.stage as AnimalProperties['reproductiveStage']) ?? defaultProps.reproductiveStage,
      ageFromDays: project.animalProperties?.ageMonths !== undefined
        ? Math.max(0, Math.round(project.animalProperties.ageMonths * 30))
        : defaultProps.ageFromDays,
      ageToDays: project.animalProperties?.ageMonths !== undefined
        ? Math.max(0, Math.round(project.animalProperties.ageMonths * 30))
        : defaultProps.ageToDays,
    };

    const localItems = project.items.map((item) => ({
      id: `local-${++itemCounter}`,
      feed: resolveFeedForProjectItem(feedCatalog, item),
      amount_kg: item.amountKg,
      is_locked: item.isLocked,
    }));

    set({
      currentRationId: null,
      currentProjectPath: path,
      currentProjectName: project.name,
      currentProjectCreatedAt: project.createdAt,
      animalGroupId: groupId,
      animalProperties,
      animalCount: Math.max(1, project.animalCount || 1),
      activeView: groupId,
      localItems,
      nutrients: recalculate(localItems),
      optimizationFeedback: null,
      customNorms: project.customNorms ?? null,
      activeNormPresetId: project.normPresetId ?? null,
      alternatives: [],
      alternativesCurrentIndex: 0,
      alternativesShowModal: false,
      alternativesPanelExpanded: false,
      alternativesPendingApply: false,
    });
  },

  loadPreset: (feeds, items, options) => {
    const newItems: LocalRationItem[] = [];
    for (const item of items) {
      const feed = findFeedInCatalogByKnownName(feeds, item.feedName)
        ?? feeds.find((candidate) => candidate.name_ru === item.feedName || candidate.name_en === item.feedName);
      if (feed) {
        newItems.push({
          id: `local-${++itemCounter}`,
          feed,
          amount_kg: item.kgPerDay,
          is_locked: false,
        });
      }
    }

    const nextProperties = options?.animalProperties ?? get().animalProperties;
    const nextGroupId = options?.groupId ?? resolveAnimalGroupId(nextProperties);

    set({
      localItems: newItems,
      nutrients: recalculate(newItems),
      animalProperties: nextProperties,
      animalCount: 1,
      animalGroupId: nextGroupId,
      activeView: options?.activeView ?? nextGroupId,
      optimizationFeedback: null,
      currentRationId: null,
      currentProjectPath: null,
      currentProjectName: null,
      currentProjectCreatedAt: null,
      activeNormPresetId: options?.normPresetId ?? null,
      customNorms: null,
      alternatives: [],
      alternativesCurrentIndex: 0,
      alternativesShowModal: false,
      alternativesPanelExpanded: false,
      alternativesPendingApply: false,
    });
  },

  setAlternatives: (result, options) =>
    set({
      alternatives: [result.primary, ...result.alternatives],
      alternativesCurrentIndex: 0,
      alternativesShowModal: result.alternatives.length > 0,
      alternativesPanelExpanded: false,
      alternativesPendingApply: options?.pendingApply ?? false,
    }),

  selectAlternative: (index, feedCatalog) =>
    set((state) => {
      const solution = state.alternatives[index];
      if (!solution) return state;

      const newItems: LocalRationItem[] = [];
      for (const optItem of solution.feeds) {
        const feed = feedCatalog.find((f) => f.id === optItem.feed_id);
        if (feed) {
          newItems.push({
            id: `optimized-${optItem.feed_id}-${Date.now()}`,
            feed,
            amount_kg: optItem.amount_kg,
            is_locked: false, // Wait, original implementation kept locks? Alternatives don't have locks. We can just set it to false.
          });
        }
      }

      return {
        alternativesCurrentIndex: index,
        localItems: newItems,
        nutrients: solution.nutrients,
        alternativesPendingApply: false,
        optimizationFeedback: state.optimizationFeedback ? {
          ...state.optimizationFeedback,
          solution: {
            ...state.optimizationFeedback.solution,
            items: solution.feeds,
            nutrient_summary: solution.nutrients,
            cost_per_day: solution.cost,
            optimization_status: solution.optimization_status,
            applied_strategy: solution.applied_strategy,
            warnings: solution.warnings,
          },
        } : null,
      };
    }),

  toggleAlternativesModal: () =>
    set((state) => ({
      alternativesShowModal: !state.alternativesShowModal,
    })),

  toggleAlternativesPanel: () =>
    set((state) => ({
      alternativesPanelExpanded: !state.alternativesPanelExpanded,
    })),

  clearAlternatives: () =>
    set({
      alternatives: [],
      alternativesCurrentIndex: 0,
      alternativesShowModal: false,
      alternativesPanelExpanded: false,
      alternativesPendingApply: false,
    }),

  toggleFarmBucketFeed: (feedId) => {
    const bucket = new Set(get().farmBucket);
    if (bucket.has(feedId)) {
      bucket.delete(feedId);
    } else {
      bucket.add(feedId);
    }
    saveFarmBucket(bucket);
    set({ farmBucket: bucket });
  },

  addToFarmBucket: (feedIds) => {
    const bucket = new Set(get().farmBucket);
    for (const id of feedIds) bucket.add(id);
    saveFarmBucket(bucket);
    set({ farmBucket: bucket });
  },

  removeFromFarmBucket: (feedIds) => {
    const bucket = new Set(get().farmBucket);
    for (const id of feedIds) bucket.delete(id);
    saveFarmBucket(bucket);
    set({ farmBucket: bucket });
  },

  setFarmBucketActive: (active) => set({ farmBucketActive: active }),

  clearFarmBucket: () => {
    saveFarmBucket(new Set());
    set({ farmBucket: new Set(), farmBucketActive: false });
  },
}));
export function calculateLocalNutrients(items: LocalRationItem[]): NutrientSummary {
  const summary: NutrientSummary = {
    total_weight_kg: 0,
    total_dm_kg: 0,
    energy_eke: 0,
    energy_oe_cattle: 0,
    energy_oe_pig: 0,
    energy_oe_poultry: 0,
    crude_protein: 0,
    dig_protein_cattle: 0,
    dig_protein_pig: 0,
    dig_protein_poultry: 0,
    lysine: 0,
    methionine_cystine: 0,
    crude_fat: 0,
    crude_fiber: 0,
    starch: 0,
    sugar: 0,
    calcium: 0,
    phosphorus: 0,
    magnesium: 0,
    potassium: 0,
    sodium: 0,
    sulfur: 0,
    iron: 0,
    copper: 0,
    zinc: 0,
    manganese: 0,
    cobalt: 0,
    iodine: 0,
    ca_p_ratio: 0,
    vit_d3: 0,
    vit_e: 0,
    carotene: 0,
    dm_pct: 0,
    cp_pct_dm: 0,
    dig_protein_cattle_pct_cp: 0,
    starch_pct_dm: 0,
  };

  for (const item of items) {
    const feed = item.feed;
    const kg = item.amount_kg;
    const dm_pct = feed.dry_matter ?? 86;
    const dm_kg = kg * dm_pct / 100;

    summary.total_weight_kg += kg;
    summary.total_dm_kg += dm_kg;
    summary.energy_oe_cattle += (feed.energy_oe_cattle ?? 0) * dm_kg;
    summary.energy_oe_pig += (feed.energy_oe_pig ?? 0) * dm_kg;
    summary.energy_oe_poultry += (feed.energy_oe_poultry ?? 0) * dm_kg;
    summary.energy_eke += (feed.energy_oe_cattle ?? 0) * dm_kg / 10.5;
    summary.crude_protein += (feed.crude_protein ?? 0) * kg;
    summary.dig_protein_cattle += (feed.dig_protein_cattle ?? 0) * kg;
    summary.dig_protein_pig += (feed.dig_protein_pig ?? 0) * kg;
    summary.dig_protein_poultry += (feed.dig_protein_poultry ?? 0) * kg;
    summary.lysine += (feed.lysine ?? 0) * kg;
    summary.methionine_cystine += (feed.methionine_cystine ?? 0) * kg;
    summary.crude_fat += (feed.crude_fat ?? 0) * kg;
    summary.crude_fiber += (feed.crude_fiber ?? 0) * kg;
    summary.starch += (feed.starch ?? 0) * kg;
    summary.sugar += (feed.sugar ?? 0) * kg;
    summary.calcium += (feed.calcium ?? 0) * kg;
    summary.phosphorus += (feed.phosphorus ?? 0) * kg;
    summary.magnesium += (feed.magnesium ?? 0) * kg;
    summary.potassium += (feed.potassium ?? 0) * kg;
    summary.sodium += (feed.sodium ?? 0) * kg;
    summary.sulfur += (feed.sulfur ?? 0) * kg;
    summary.iron += (feed.iron ?? 0) * kg;
    summary.copper += (feed.copper ?? 0) * kg;
    summary.zinc += (feed.zinc ?? 0) * kg;
    summary.manganese += (feed.manganese ?? 0) * kg;
    summary.cobalt += (feed.cobalt ?? 0) * kg;
    summary.iodine += (feed.iodine ?? 0) * kg;
    summary.vit_d3 += (feed.vit_d3 ?? 0) * kg;
    summary.vit_e += (feed.vit_e ?? 0) * kg;
    summary.carotene += (feed.carotene ?? 0) * kg;
  }

  if (summary.total_weight_kg > 0 && summary.total_dm_kg > 0) {
    summary.dm_pct = (summary.total_dm_kg / summary.total_weight_kg) * 100;
    summary.cp_pct_dm = (summary.crude_protein / 1000 / summary.total_dm_kg) * 100;
    summary.starch_pct_dm = (summary.starch / 1000 / summary.total_dm_kg) * 100;
  }

  if (summary.crude_protein > 0) {
    summary.dig_protein_cattle_pct_cp = (summary.dig_protein_cattle / summary.crude_protein) * 100;
  }

  if (summary.phosphorus > 0) {
    summary.ca_p_ratio = summary.calcium / summary.phosphorus;
  }

  return summary;
}

