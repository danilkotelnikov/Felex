import { feedsApi, rationsApi } from './api';
import type { Feed } from '@/types/feed';

interface SyncLocalItem {
  id: string;
  feed: Feed;
  amount_kg: number;
  is_locked: boolean;
}

interface EnsureBackendRationArgs {
  currentRationId: number | null;
  animalGroupId: string;
  animalCount: number;
  currentProjectName: string | null;
  localItems: SyncLocalItem[];
}

interface EnsureBackendRationResult {
  rationId: number;
  backendFeedIdsByLocalId: Map<string, number>;
}

function normalizeName(value: string | undefined) {
  return (value || '')
    .normalize('NFKC')
    .trim()
    .toLowerCase()
    .replace(/\u0451/g, '\u0435')
    .replace(/[^0-9a-z\u0400-\u04ff]+/gi, ' ')
    .replace(/\s+/g, ' ');
}

function namesMatch(left: Feed, right: Feed) {
  const targetNames = new Set([normalizeName(left.name_ru), normalizeName(left.name_en)]);
  const candidateNames = [normalizeName(right.name_ru), normalizeName(right.name_en)];

  return candidateNames.some((name) => name && targetNames.has(name));
}

function resolveBackendFeedId(feed: Feed, backendFeeds: Feed[]) {
  if (feed.source_id) {
    const bySourceId = backendFeeds.find((candidate) => candidate.source_id === feed.source_id);
    if (bySourceId?.id) {
      return bySourceId.id;
    }
  }

  const byId = backendFeeds.find((candidate) => candidate.id === feed.id);
  if (
    byId?.id &&
    ((feed.source_id && byId.source_id === feed.source_id) || namesMatch(feed, byId))
  ) {
    return byId.id;
  }

  const byName = backendFeeds.find((candidate) => {
    if (feed.source_id && candidate.source_id === feed.source_id) {
      return true;
    }

    return namesMatch(feed, candidate);
  });

  return byName?.id ?? null;
}

function resolveFeedIds(localItems: SyncLocalItem[], backendFeeds: Feed[]) {
  const missingFeeds: string[] = [];
  const backendFeedIdsByLocalId = new Map<string, number>();

  for (const item of localItems) {
    const backendFeedId = resolveBackendFeedId(item.feed, backendFeeds);
    if (!backendFeedId) {
      missingFeeds.push(item.feed.name_ru);
      continue;
    }

    backendFeedIdsByLocalId.set(item.id, backendFeedId);
  }

  return { missingFeeds, backendFeedIdsByLocalId };
}

export async function ensureBackendRation(args: EnsureBackendRationArgs): Promise<EnsureBackendRationResult> {
  let backendFeeds = (await feedsApi.list({ limit: 5000 })).data ?? [];
  let resolution = resolveFeedIds(args.localItems, backendFeeds);

  if (backendFeeds.length === 0 || resolution.missingFeeds.length > 0) {
    await feedsApi.sync();
    backendFeeds = (await feedsApi.list({ limit: 5000 })).data ?? [];
    resolution = resolveFeedIds(args.localItems, backendFeeds);
  }

  if (resolution.missingFeeds.length > 0) {
    throw new Error(`Feeds not found in reference DB: ${resolution.missingFeeds.slice(0, 5).join(', ')}`);
  }

  let rationId = args.currentRationId;
  if (!rationId) {
    const created = await rationsApi.create({
      name: args.currentProjectName ?? 'Workspace ration',
      animal_group_id: args.animalGroupId,
      animal_count: args.animalCount,
      description: 'Workspace synchronization snapshot',
    });
    rationId = created.data;
  }

  await rationsApi.update(rationId, {
    name: args.currentProjectName ?? 'Workspace ration',
    animal_group_id: args.animalGroupId,
    animal_count: args.animalCount,
    description: 'Workspace synchronization snapshot',
    items: args.localItems.map((item) => ({
      feed_id: resolution.backendFeedIdsByLocalId.get(item.id)!,
      amount_kg: item.amount_kg,
      is_locked: item.is_locked,
    })),
  });

  return {
    rationId,
    backendFeedIdsByLocalId: resolution.backendFeedIdsByLocalId,
  };
}
