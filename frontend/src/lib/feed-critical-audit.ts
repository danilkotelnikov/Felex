import type { TFunction } from 'i18next';

import { getNutrientLabel, resolveNutrientLanguage } from '@/lib/nutrient-registry';
import type { FeedCriticalNutrientKey, FeedProfileStatus } from '@/types/feed';

export function criticalCoverageBadgeVariant(
  status: FeedProfileStatus | null | undefined,
): 'success' | 'warning' | 'error' {
  switch (status) {
    case 'complete':
      return 'success';
    case 'partial':
      return 'warning';
    default:
      return 'error';
  }
}

export function localizeCriticalNutrientKey(
  key: FeedCriticalNutrientKey,
  t: TFunction,
  language: string | null | undefined,
): string {
  if (key === 'dry_matter') {
    return t('feedLibrary.criticalKey.dry_matter');
  }

  return getNutrientLabel(key, resolveNutrientLanguage(language));
}

export function localizeCriticalContext(
  contextId: string,
  t: TFunction,
): string {
  return t(`settings.criticalContext.${contextId}`);
}
