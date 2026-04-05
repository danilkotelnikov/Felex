export interface FeedPriceKeyRecord {
  feed_id: number;
  source_id?: string | null;
  category: string;
  subcategory?: string | null;
  region?: string | null;
  aliases_ru: string[];
  aliases_en: string[];
  search_terms: string[];
}

let priceKeysCache: FeedPriceKeyRecord[] | null = null;
let priceKeysPromise: Promise<FeedPriceKeyRecord[]> | null = null;

export async function loadGeneratedFeedPriceKeys(): Promise<FeedPriceKeyRecord[]> {
  if (priceKeysCache) {
    return priceKeysCache;
  }

  if (!priceKeysPromise) {
    priceKeysPromise = import('@/generated/feed-price-keys.generated.json').then((module) => {
      priceKeysCache = module.default as FeedPriceKeyRecord[];
      return priceKeysCache;
    });
  }

  return priceKeysPromise;
}

export function findFeedPriceKey(
  rows: FeedPriceKeyRecord[],
  feedId: number,
  sourceId?: string | null,
): FeedPriceKeyRecord | null {
  if (sourceId) {
    const bySourceId = rows.find((row) => row.source_id === sourceId);
    if (bySourceId) {
      return bySourceId;
    }
  }
  return rows.find((row) => row.feed_id === feedId) ?? null;
}

