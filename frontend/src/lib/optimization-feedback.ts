import type { TFunction } from 'i18next';
import type { Feed } from '@/types/feed';
import type { AutoAddedFeed, DietSolution, RelaxedConstraintType, RelaxedTarget } from '@/types/ration';
import {
  getNutrientLabel,
  getNutrientUnit,
  resolveNutrientKey,
  resolveNutrientLanguage,
} from '@/lib/nutrient-registry';

interface RelaxedTargetDisplay {
  label: string;
  unit: string;
}

const WORKFLOW_NOTE_KEYS: Record<string, string> = {
  'Selected-feeds-only mode ignores library repair and uses tiered balance.':
    'optimize.noteSelectedOnlyUsesBalance',
  'Library-assisted construction starts with repair balance before any fixed or cost-only pass.':
    'optimize.noteLibraryStartsWithRepair',
  'Built a starter ration from the library template.':
    'optimize.noteStarterBuilt',
  'Selected feeds alone could not satisfy the reference. Allow library completion to add missing ingredients.':
    'optimize.noteSelectedOnlyNeedsLibrary',
  'Library-assisted construction still could not reach a feasible ration with the current feed database.':
    'optimize.noteLibraryStillInsufficient',
  'Returned the closest achievable ration with the selected feeds. Some targets remain relaxed.':
    'optimize.noteBestSelectedOnly',
  'Allow library completion if you want the optimizer to add missing ingredients and close the remaining gaps.':
    'optimize.noteAllowLibraryCompletion',
  'Completed the ration from the library as far as possible. Some targets remain relaxed.':
    'optimize.noteBestCompleteLibrary',
  'Built the closest achievable ration from the current library. Some targets remain relaxed.':
    'optimize.noteBestBuildLibrary',
  'Balanced the current ingredient set without adding library feeds.':
    'optimize.noteBalancedSelectedOnly',
  'Completed the ration from the library and balanced the result.':
    'optimize.noteBalancedCompleteLibrary',
  'Constructed a ration from the library and balanced it.':
    'optimize.noteBalancedBuildLibrary',
  'Used the selected reference set while constructing the ration.':
    'optimize.noteSelectedReferenceUsed',
  'Starter ration could not be assembled from the current feed library.':
    'optimize.noteStarterUnavailable',
};

const REASON_KEYS: Record<string, string> = {
  'Added as the missing roughage source.': 'optimize.reasonMissingRoughage',
  'Added as the missing succulent base.': 'optimize.reasonMissingSucculent',
  'Added as the missing concentrate source.': 'optimize.reasonMissingConcentrate',
  'Added as the missing protein source.': 'optimize.reasonMissingProtein',
  'Added as the missing animal-origin protein source.': 'optimize.reasonMissingAnimalProtein',
  'Added as the missing mineral source.': 'optimize.reasonMissingMineral',
  'Added as the missing vitamin-mineral premix.': 'optimize.reasonMissingPremix',
  'Added as a missing supporting ingredient.': 'optimize.reasonMissingSupport',
  'Supports the energy deficit.': 'optimize.reasonEnergy',
  'Supports the protein deficit.': 'optimize.reasonProtein',
  'Supports lysine coverage.': 'optimize.reasonLysine',
  'Supports sulfur amino acid coverage.': 'optimize.reasonMethionineCystine',
  'Supports structural fiber coverage.': 'optimize.reasonFiber',
  'Supports starch balance.': 'optimize.reasonStarch',
  'Supports calcium coverage.': 'optimize.reasonCalcium',
  'Supports phosphorus coverage.': 'optimize.reasonPhosphorus',
  'Supports vitamin D3 coverage.': 'optimize.reasonVitaminD3',
  'Supports vitamin E coverage.': 'optimize.reasonVitaminE',
  'Supports the remaining nutrient gap.': 'optimize.reasonRemainingGap',
};

const SCREENING_LABEL_KEYS: Record<string, string> = {
  energy: 'optimize.labelEnergy',
  protein: 'optimize.labelProtein',
  'digestible protein': 'optimize.labelDigestibleProtein',
  lysine: 'optimize.labelLysine',
  'methionine+cystine': 'optimize.labelMethionineCystine',
  calcium: 'optimize.labelCalcium',
  phosphorus: 'optimize.labelPhosphorus',
  'vitamin D3': 'optimize.labelVitaminD3',
  'vitamin E': 'optimize.labelVitaminE',
  starch: 'optimize.labelStarch',
  'crude fiber': 'optimize.labelCrudeFiber',
};

const GROUP_LABEL_KEYS: Record<string, string> = {
  roughage: 'optimize.groupRoughage',
  succulent: 'optimize.groupSucculent',
  concentrate: 'optimize.groupConcentrate',
  protein: 'optimize.groupProtein',
  animal_origin: 'optimize.groupAnimalOrigin',
  mineral: 'optimize.groupMineral',
  premix: 'optimize.groupPremix',
  vitamin: 'optimize.groupVitamin',
  other: 'optimize.groupOther',
};

function prettifyMetricKey(key: string): string {
  return key
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function relaxedTargetSummaryKey(type: RelaxedConstraintType) {
  switch (type) {
    case 'min':
      return 'optimize.relaxedMin';
    case 'max':
      return 'optimize.relaxedMax';
    default:
      return 'optimize.relaxedTarget';
  }
}

export function localizeOptimizationStatus(
  status: DietSolution['optimization_status'],
  t: TFunction
) {
  switch (status) {
    case 'Optimal':
      return t('optimize.statusOptimal');
    case 'Feasible':
      return t('optimize.statusFeasible');
    case 'Infeasible':
      return t('optimize.statusInfeasible');
    case 'Unbounded':
      return t('optimize.statusUnbounded');
    default:
      return t('optimize.statusError');
  }
}

function localizeScreeningLabel(label: string, t: TFunction) {
  const key = SCREENING_LABEL_KEYS[label];
  return key ? t(key) : label;
}

function localizeGroupLabel(label: string, t: TFunction) {
  const key = GROUP_LABEL_KEYS[label];
  return key ? t(key) : label;
}

export function localizeWorkflowNote(note: string, t: TFunction) {
  if (note.startsWith('optimize.')) {
    return t(note);
  }

  const directKey = WORKFLOW_NOTE_KEYS[note];
  if (directKey) {
    return t(directKey);
  }

  let match = note.match(
    /^Completed the current ration with (\d+) starter feeds? from the library\.$/
  );
  if (match) {
    return t('optimize.noteStarterFeedsAdded', { count: Number(match[1]) });
  }

  match = note.match(/^Constructed a starter ration from (\d+) library candidate sets?\.$/);
  if (match) {
    return t('optimize.noteConstructorTriedSets', { count: Number(match[1]) });
  }

  match = note.match(
    /^Auto-added (\d+) feeds? from the library\. Review the added-feed notes below for the repair roles\.$/
  );
  if (match) {
    return t('optimize.noteAutoAddedFeeds', { count: Number(match[1]) });
  }

  match = note.match(/^No starter feed found for ([a-z_]+)\.$/);
  if (match) {
    return t('optimize.noteNoStarterGroup', { group: localizeGroupLabel(match[1], t) });
  }

  return note;
}

export function localizeOptimizationReason(reason: string, t: TFunction) {
  if (reason.startsWith('optimize.')) {
    return t(reason);
  }

  const directKey = REASON_KEYS[reason];
  if (directKey) {
    return t(directKey);
  }

  const match = reason.match(/^(.+) support(?: for (cattle|swine|poultry))?\.$/);
  if (match) {
    return t('optimize.reasonSupport', {
      nutrient: localizeScreeningLabel(match[1], t),
    });
  }

  return reason;
}

export function matchAutoAddedFeed(
  feed: Pick<Feed, 'id' | 'name_ru' | 'name_en'>,
  autoAddedFeeds: AutoAddedFeed[]
): AutoAddedFeed | null {
  const normalizedRu = feed.name_ru.trim().toLowerCase();
  const normalizedEn = feed.name_en?.trim().toLowerCase();

  return autoAddedFeeds.find((entry) => {
    if (entry.feed_id === feed.id) {
      return true;
    }

    const normalizedEntryName = entry.feed_name.trim().toLowerCase();
    return normalizedEntryName === normalizedRu || (!!normalizedEn && normalizedEntryName === normalizedEn);
  }) ?? null;
}

export function getRelaxedTargetDisplay(
  target: RelaxedTarget,
  language: string
): RelaxedTargetDisplay {
  const nutrientLanguage = resolveNutrientLanguage(language);
  const resolvedKey = resolveNutrientKey(target.key);
  const label = getNutrientLabel(resolvedKey, nutrientLanguage);
  const unit = getNutrientUnit(resolvedKey, nutrientLanguage);

  if (label !== prettifyMetricKey(resolvedKey) || unit) {
    return {
      label,
      unit,
    };
  }

  return {
    label: prettifyMetricKey(target.key),
    unit: '',
  };
}
