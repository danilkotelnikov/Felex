import type {
  Feed,
  FeedProfileSectionAudit,
  FeedProfileSectionKey,
  FeedProfileSectionStatus,
  FeedProfileStatus,
  FeedSourceKind,
  FeedTranslationStatus,
} from '@/types/feed';
import { getFeedCategoryLabel, getFeedSubcategoryLabel } from '@/lib/feed-categories';

export type FeedUiLanguage = 'ru' | 'en';
export type FeedSourceKey = FeedSourceKind;
export type FeedTranslationKey = FeedTranslationStatus;

export interface FeedProfileAudit {
  sourceKey: FeedSourceKey;
  translationKey: FeedTranslationKey;
  overallStatus: FeedProfileStatus;
  sections: FeedProfileSectionAudit[];
}

function normalizeText(value: string | null | undefined): string {
  return (value ?? '')
    .trim()
    .toLowerCase()
    .replace(/\u0451/g, '\u0435')
    .replace(/([\p{L}])(\d)/gu, '$1 $2')
    .replace(/(\d)([\p{L}])/gu, '$1 $2')
    .replace(/[\s()+/\\,:;.-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function compactText(value: string | null | undefined): string {
  return normalizeText(value).replace(/\s+/g, '');
}

const MATCH_STOPWORDS = new Set([
  'для',
  'the',
  'and',
  'feed',
  'корм',
  'кормовой',
  'кормовая',
  'кормовое',
  'кормовые',
]);

function canonicalToken(token: string): string {
  if (!token) {
    return token;
  }
  if (MATCH_STOPWORDS.has(token)) {
    return '';
  }
  if (token.startsWith('дроблен') || token.startsWith('дерт')) {
    return 'дерть';
  }
  if (token.startsWith('ячмен')) {
    return 'ячмень';
  }
  if (token.startsWith('пшенич')) {
    return 'пшеница';
  }
  if (token.startsWith('овс') || token === 'овес' || token === 'овёс') {
    return 'овес';
  }
  if (token.startsWith('свеклович')) {
    return 'свекловичный';
  }
  if (token.startsWith('поваренн')) {
    return 'поваренная';
  }
  if (token.startsWith('премикс')) {
    return 'премикс';
  }
  if (token.startsWith('жом')) {
    return 'жом';
  }
  return token;
}

function tokenizeParts(parts: Array<string | null | undefined>): Set<string> {
  const tokens = parts
    .flatMap((part) => normalizeText(part).match(/[\p{L}\d%]+/gu) ?? [])
    .map(canonicalToken)
    .filter(Boolean);
  return new Set(tokens);
}

function hasValue(value: number | null | undefined): boolean {
  return typeof value === 'number' && Number.isFinite(value) && Math.abs(value) > 1e-9;
}

function countPresent(feed: Feed, keys: Array<keyof Feed>): number {
  return keys.reduce((count, key) => count + (hasValue(feed[key] as number | undefined) ? 1 : 0), 0);
}

function sectionAudit(
  key: FeedProfileSectionKey,
  present: number,
  expected: number,
): FeedProfileSectionAudit | null {
  if (expected <= 0 && present <= 0) {
    return null;
  }

  let status: FeedProfileSectionStatus = 'missing';
  if (present >= expected && present > 0) {
    status = 'present';
  } else if (present > 0) {
    status = 'partial';
  }

  return { key, status, present, expected };
}

function sectionRequirements(category: string | undefined): Record<FeedProfileSectionKey, number> {
  switch ((category ?? 'other').trim()) {
    case 'grain':
    case 'concentrate':
      return { energy: 3, protein: 2, fiber: 1, minerals: 2, vitamins: 0 };
    case 'oilseed_meal':
    case 'protein':
    case 'animal_origin':
      return { energy: 2, protein: 3, fiber: 1, minerals: 2, vitamins: 0 };
    case 'roughage':
    case 'silage':
      return { energy: 2, protein: 1, fiber: 2, minerals: 2, vitamins: 0 };
    case 'succulent':
      return { energy: 2, protein: 1, fiber: 1, minerals: 2, vitamins: 0 };
    case 'mineral':
      return { energy: 0, protein: 0, fiber: 0, minerals: 2, vitamins: 0 };
    case 'premix':
      return { energy: 0, protein: 0, fiber: 0, minerals: 1, vitamins: 1 };
    case 'additive':
      return { energy: 1, protein: 0, fiber: 0, minerals: 0, vitamins: 0 };
    default:
      return { energy: 1, protein: 1, fiber: 0, minerals: 1, vitamins: 0 };
  }
}

export function resolveFeedLanguage(language: string | null | undefined): FeedUiLanguage {
  return language?.startsWith('en') ? 'en' : 'ru';
}

export function hasTranslatedFeedName(feed: Pick<Feed, 'name_ru' | 'name_en'>): boolean {
  const ru = normalizeText(feed.name_ru);
  const en = normalizeText(feed.name_en);
  return en.length > 0 && en !== ru;
}

function englishContextLabel(feed: Pick<Feed, 'subcategory' | 'source_subcategory_en' | 'category'>): string | null {
  if (feed.source_subcategory_en?.trim()) {
    return feed.source_subcategory_en.trim();
  }
  return getFeedSubcategoryLabel(feed.subcategory, 'en') || getFeedCategoryLabel(feed.category ?? 'other', 'en');
}

export function getFeedDisplayName(
  feed: Pick<Feed, 'id' | 'name_ru' | 'name_en' | 'subcategory' | 'source_subcategory_en' | 'category'>,
  language: FeedUiLanguage,
): string {
  if (language === 'en' && hasTranslatedFeedName(feed)) {
    return feed.name_en!.trim();
  }

  if (language === 'en') {
    const contextLabel = englishContextLabel(feed);
    if (contextLabel) {
      return `${contextLabel} · ${feed.name_ru.trim()}`;
    }
  }

  return feed.name_ru?.trim() || feed.name_en?.trim() || `Feed #${feed.id}`;
}

export function getFeedSecondaryName(
  feed: Pick<Feed, 'name_ru' | 'name_en' | 'subcategory' | 'source_subcategory_en' | 'category'>,
  language: FeedUiLanguage,
): string | null {
  if (language === 'en') {
    return hasTranslatedFeedName(feed) ? feed.name_ru.trim() : null;
  }

  return hasTranslatedFeedName(feed) ? feed.name_en!.trim() : null;
}

export function getFeedDisplayNameFromCatalog(
  feedId: number | null | undefined,
  fallbackName: string | null | undefined,
  catalog: Array<Pick<Feed, 'id' | 'name_ru' | 'name_en' | 'subcategory' | 'source_subcategory_en' | 'category'>>,
  language: FeedUiLanguage,
): string {
  const byId = typeof feedId === 'number'
    ? catalog.find((feed) => feed.id === feedId)
    : undefined;

  if (byId) {
    return getFeedDisplayName(byId, language);
  }

  if (fallbackName?.trim()) {
    const byKnownName = findFeedInCatalogByKnownName(catalog, fallbackName);
    if (byKnownName) {
      return getFeedDisplayName(byKnownName, language);
    }

    return fallbackName.trim();
  }

  return typeof feedId === 'number' ? `Feed #${feedId}` : 'Feed';
}

export function scoreFeedKnownNameMatch(
  feed: Pick<Feed, 'name_ru' | 'name_en' | 'subcategory'>,
  candidate: string,
): number {
  const normalizedCandidate = normalizeText(candidate);
  const compactCandidate = compactText(candidate);
  if (!normalizedCandidate) {
    return 0;
  }

  const normalizedNameRu = normalizeText(feed.name_ru);
  const normalizedNameEn = normalizeText(feed.name_en);
  const compactNameRu = compactText(feed.name_ru);
  const compactNameEn = compactText(feed.name_en);
  if (normalizedNameRu === normalizedCandidate || normalizedNameEn === normalizedCandidate) {
    return 100;
  }
  if (compactCandidate && (compactNameRu === compactCandidate || compactNameEn === compactCandidate)) {
    return 95;
  }

  const candidateTokens = tokenizeParts([candidate]);
  if (candidateTokens.size === 0) {
    return 0;
  }

  const feedTokens = tokenizeParts([feed.name_ru, feed.name_en, feed.subcategory]);
  if (feedTokens.size === 0) {
    return 0;
  }

  let overlap = 0;
  for (const token of candidateTokens) {
    if (feedTokens.has(token)) {
      overlap += 1;
    }
  }

  if (overlap === 0) {
    return 0;
  }

  const candidateCoverage = overlap / candidateTokens.size;
  const feedCoverage = overlap / feedTokens.size;
  if (candidateCoverage === 1 && feedCoverage === 1) {
    return 90;
  }
  if (candidateCoverage === 1 && candidateTokens.size >= 2) {
    return Math.max(65, 80 - Math.max(feedTokens.size - candidateTokens.size, 0));
  }
  if (candidateCoverage >= 0.75 && overlap >= 2 && feedCoverage >= 0.4) {
    return 60 + overlap;
  }
  if (candidateCoverage >= 0.35 && overlap >= 2 && feedCoverage >= 0.1) {
    return 62 + Math.min(overlap, 3);
  }
  if (candidateCoverage >= 0.6 && overlap >= 3) {
    return 45 + overlap;
  }
  return 0;
}

export function matchFeedByKnownName(
  feed: Pick<Feed, 'name_ru' | 'name_en' | 'subcategory'>,
  candidate: string,
): boolean {
  return scoreFeedKnownNameMatch(feed, candidate) > 0;
}

export function findFeedInCatalogByKnownName<T extends Pick<Feed, 'name_ru' | 'name_en' | 'subcategory'>>(
  catalog: T[],
  candidate: string,
): T | undefined {
  let bestFeed: T | undefined;
  let bestScore = 0;

  for (const feed of catalog) {
    const score = scoreFeedKnownNameMatch(feed, candidate);
    if (score > bestScore) {
      bestScore = score;
      bestFeed = feed;
    }
  }

  return bestScore >= 60 ? bestFeed : undefined;
}

function getBackendProfileAudit(feed: Feed): FeedProfileAudit | null {
  if (!feed.source_kind || !feed.translation_status || !feed.profile_status) {
    return null;
  }

  return {
    sourceKey: feed.source_kind,
    translationKey: feed.translation_status,
    overallStatus: feed.profile_status,
    sections: (feed.profile_sections ?? []).map((section) => ({
      key: section.key,
      status: section.status,
      present: section.present,
      expected: section.expected,
    })),
  };
}

export function getFeedProfileAudit(feed: Feed): FeedProfileAudit {
  const backendAudit = getBackendProfileAudit(feed);
  if (backendAudit) {
    return backendAudit;
  }

  const requirements = sectionRequirements(feed.category);
  const energy = countPresent(feed, [
    'dry_matter',
    'koe',
    'energy_oe_cattle',
    'energy_oe_pig',
    'energy_oe_poultry',
  ]);
  const protein = countPresent(feed, [
    'crude_protein',
    'dig_protein_cattle',
    'dig_protein_pig',
    'dig_protein_poultry',
    'lysine',
    'methionine_cystine',
  ]);
  const fiber = countPresent(feed, ['crude_fiber', 'crude_fat', 'starch', 'sugar']);
  const minerals = countPresent(feed, [
    'calcium',
    'phosphorus',
    'magnesium',
    'potassium',
    'sodium',
    'sulfur',
    'iron',
    'copper',
    'zinc',
    'manganese',
    'cobalt',
    'iodine',
  ]);
  const vitamins = countPresent(feed, [
    'carotene',
    'vit_d3',
    'vit_e',
  ]);

  const sections = [
    sectionAudit('energy', energy, requirements.energy),
    sectionAudit('protein', protein, requirements.protein),
    sectionAudit('fiber', fiber, requirements.fiber),
    sectionAudit('minerals', minerals, requirements.minerals),
    sectionAudit('vitamins', vitamins, requirements.vitamins),
  ].filter((section): section is FeedProfileSectionAudit => Boolean(section));

  const requiredSections = sections.filter((section) => section.expected > 0);
  const presentRequired = requiredSections.filter((section) => section.status === 'present').length;
  const missingRequired = requiredSections.filter((section) => section.status === 'missing').length;
  const overallStatus: FeedProfileStatus =
    requiredSections.length === 0
      ? (sections.some((section) => section.status !== 'missing') ? 'partial' : 'limited')
      : missingRequired === 0
        ? 'complete'
        : presentRequired >= Math.max(requiredSections.length - 1, 1)
          ? 'partial'
          : 'limited';

  return {
    sourceKey: feed.is_custom
      ? 'custom'
      : (feed.source_id ?? '').startsWith('seed:normalized-db:')
        ? 'normalized'
        : (feed.source_id ?? '').startsWith('seed:')
          ? 'curated'
          : 'imported',
    translationKey: hasTranslatedFeedName(feed) ? 'ready' : 'source_only',
    overallStatus,
    sections,
  };
}
