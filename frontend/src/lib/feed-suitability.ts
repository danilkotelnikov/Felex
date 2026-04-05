import type { TFunction } from 'i18next';
import type { Feed, FeedSuitabilityStatus } from '@/types/feed';

const FEED_SUITABILITY_NOTE_KEYS: Record<string, string> = {
  'Species-targeted formula matches the current animal type.': 'feedLibrary.suitabilityNoteSpeciesMatch',
  'Species-targeted formula does not match the current animal type.': 'feedLibrary.suitabilityNoteSpeciesMismatch',
  'Stage-targeted formula matches the current production phase.': 'feedLibrary.suitabilityNoteStageMatch',
  'Stage-targeted formula does not match the current production phase.': 'feedLibrary.suitabilityNoteStageMismatch',
  'Stage-targeted formula requires production-phase confirmation.': 'feedLibrary.suitabilityNoteStageConfirm',
  'Cattle-targeted formula matches the current cattle class.': 'feedLibrary.suitabilityNoteCattleClassMatch',
  'Cattle-targeted formula does not match the current cattle class.': 'feedLibrary.suitabilityNoteCattleClassMismatch',
  'Cattle-targeted formula requires cattle-class confirmation.': 'feedLibrary.suitabilityNoteCattleClassConfirm',
  'Layer shell grit is intended for egg-producing poultry.': 'feedLibrary.suitabilityNoteLayerShell',
  'Raw potato ingredients are excluded from the poultry candidate set.': 'feedLibrary.suitabilityNoteRawPotatoPoultry',
  'Potato ingredients require processing-state and inclusion review before use.': 'feedLibrary.suitabilityNotePotatoConditional',
  'Keep within the species-specific inclusion limit.': 'feedLibrary.suitabilityNoteMaxInclusion',
};

export function localizeFeedSuitabilityNote(note: string, t: TFunction): string {
  const key = FEED_SUITABILITY_NOTE_KEYS[note];
  return key ? t(key) : note;
}

export function feedSuitabilityBadgeVariant(status?: FeedSuitabilityStatus): 'success' | 'warning' | 'error' | 'secondary' {
  switch (status) {
    case 'appropriate':
      return 'success';
    case 'conditional':
      return 'warning';
    case 'restricted':
      return 'error';
    default:
      return 'secondary';
  }
}

export function hasContextualSuitability(feed: Feed): boolean {
  return Boolean(
    feed.suitability_status
      || feed.suitability_notes?.length
      || feed.suitability_max_inclusion_pct,
  );
}
