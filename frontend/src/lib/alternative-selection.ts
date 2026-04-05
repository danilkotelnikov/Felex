import { feedsApi, rationsApi } from '@/lib/api';
import type { Feed } from '@/types/feed';
import type { AlternativeRationSolution } from '@/types/optimization';
import type { DietSolution } from '@/types/ration';

export async function ensureAlternativeFeedCatalog(
  solution: AlternativeRationSolution,
  feedCatalog: Feed[],
): Promise<Feed[]> {
  const feedIds = new Set(feedCatalog.map((feed) => feed.id));
  const hasAllFeeds = solution.feeds.every((item) => feedIds.has(item.feed_id));
  if (hasAllFeeds) {
    return feedCatalog;
  }

  const backendFeeds = (await feedsApi.list({ limit: 5000 })).data ?? [];
  const mergedCatalog = [...feedCatalog];

  for (const feed of backendFeeds) {
    if (!feedIds.has(feed.id)) {
      feedIds.add(feed.id);
      mergedCatalog.push(feed);
    }
  }

  return mergedCatalog;
}

export async function persistAlternativeSelection(
  rationId: number,
  solution: AlternativeRationSolution,
): Promise<void> {
  await rationsApi.update(rationId, {
    items: solution.feeds.map((item) => ({
      feed_id: item.feed_id,
      amount_kg: item.amount_kg,
      is_locked: false,
    })),
  });
}

export function mergeAlternativeIntoDietSolution(
  baseSolution: DietSolution | null | undefined,
  solution: AlternativeRationSolution,
): DietSolution | null {
  if (!baseSolution) {
    return null;
  }

  return {
    ...baseSolution,
    items: solution.feeds,
    nutrient_summary: solution.nutrients,
    cost_per_day: solution.cost,
    optimization_status: solution.optimization_status,
    applied_strategy: solution.applied_strategy,
    warnings: solution.warnings,
  };
}
