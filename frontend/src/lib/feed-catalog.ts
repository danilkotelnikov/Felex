import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { feedsApi } from '@/lib/api';
import { findFeedInCatalogByKnownName } from '@/lib/feed-display';
import type { Feed } from '@/types/feed';

let generatedCatalogCache: Feed[] | null = null;
let generatedCatalogPromise: Promise<Feed[]> | null = null;
let generatedDetailCache: Map<number, Feed> | null = null;
let generatedDetailPromise: Promise<Map<number, Feed>> | null = null;

function normalizeSearch(value: string): string {
  return value.trim().toLowerCase();
}

export async function loadGeneratedFeedCatalog(): Promise<Feed[]> {
  if (generatedCatalogCache) {
    return generatedCatalogCache;
  }

  if (!generatedCatalogPromise) {
    generatedCatalogPromise = import('@/generated/feed-catalog.generated.json').then((module) => {
      generatedCatalogCache = module.default as Feed[];
      return generatedCatalogCache;
    });
  }

  return generatedCatalogPromise;
}

export async function loadGeneratedFeedDetails(): Promise<Map<number, Feed>> {
  if (generatedDetailCache) {
    return generatedDetailCache;
  }

  if (!generatedDetailPromise) {
    generatedDetailPromise = import('@/generated/feed-details.generated.json').then((module) => {
      const rows = module.default as Feed[];
      generatedDetailCache = new Map(
        rows
          .filter((feed): feed is Feed & { id: number } => typeof feed.id === 'number')
          .map((feed) => [feed.id, feed]),
      );
      return generatedDetailCache;
    });
  }

  return generatedDetailPromise;
}

export function filterFeedCatalog(catalog: Feed[], search: string): Feed[] {
  const normalizedSearch = normalizeSearch(search);
  if (!normalizedSearch) {
    return catalog;
  }

  return catalog.filter((feed) => {
    const nameRu = feed.name_ru.toLowerCase();
    const nameEn = feed.name_en?.toLowerCase() ?? '';
    return nameRu.includes(normalizedSearch) || nameEn.includes(normalizedSearch);
  });
}

export function findFeedByKnownName(catalog: Feed[], feedName: string): Feed | undefined {
  return findFeedInCatalogByKnownName(catalog, feedName);
}

export function useFeedCatalog(
  search = '',
  contextParams: { species?: string; stageContext?: string } = {},
) {
  const normalizedSearch = normalizeSearch(search);
  const generatedCatalogQuery = useQuery({
    queryKey: ['generatedFeedCatalog'],
    queryFn: loadGeneratedFeedCatalog,
    staleTime: Infinity,
  });

  const fallbackFeeds = useMemo(
    () => filterFeedCatalog(generatedCatalogQuery.data ?? [], normalizedSearch),
    [generatedCatalogQuery.data, normalizedSearch],
  );

  const query = useQuery({
    queryKey: ['feedCatalog', normalizedSearch, contextParams.species ?? null, contextParams.stageContext ?? null],
    queryFn: () =>
      feedsApi.list({
        search: normalizedSearch || undefined,
        limit: 5000,
        species: contextParams.species,
        stageContext: contextParams.stageContext,
      }),
    staleTime: 5 * 60 * 1000,
  });

  const usingFallback =
    Boolean(query.error) ||
    (!normalizedSearch &&
      !!query.data &&
      (query.data.total ?? 0) === 0 &&
      (query.data.data?.length ?? 0) === 0);

  const feeds = usingFallback ? fallbackFeeds : (query.data?.data ?? fallbackFeeds);
  const total = usingFallback ? fallbackFeeds.length : (query.data?.total ?? feeds.length);

  return {
    ...query,
    feeds,
    total,
    usingFallback,
    contextAware: !usingFallback && Boolean(contextParams.species),
    liveData: query.data ?? null,
    generatedCatalogReady: generatedCatalogQuery.status === 'success',
  };
}
