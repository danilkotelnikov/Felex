import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Check, DollarSign, Edit2, Loader2, RefreshCw, X } from 'lucide-react';
import toast from 'react-hot-toast';

import { Button } from '../ui/Button';
import { Icon } from '../ui/Icon';
import { Input } from '../ui/Input';
import { PriceProvenanceDialog, type PriceDialogRow } from './PriceProvenanceDialog';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { feedsApi, pricesApi, type FeedPrice, type PriceProvenance } from '@/lib/api';
import { getFeedCategoryLabel } from '@/lib/feed-categories';
import { loadGeneratedFeedCatalog } from '@/lib/feed-catalog';
import { openExternalUrl } from '@/lib/desktop';
import { isNotificationEnabled } from '@/lib/preferences';
import { cn } from '@/lib/utils';
import type { Feed } from '@/types/feed';

function fallbackProvenance(feed: Feed): PriceProvenance | null {
  if (feed.price_per_ton == null) {
    return null;
  }

  return {
    kind: feed.is_custom ? 'manual' : 'seed',
    is_precise_source: false,
    anchor_sources: [],
  };
}

function buildRows(catalog: Feed[], prices: FeedPrice[]): PriceDialogRow[] {
  const priceMap = new Map<number, FeedPrice>();
  for (const price of prices) {
    priceMap.set(price.feed_id, price);
  }

  return catalog.map((feed) => {
    const price = feed.id != null ? priceMap.get(feed.id) : undefined;
    return {
      feedId: feed.id ?? 0,
      feedName: feed.name_ru || feed.name_en || `Feed #${feed.id ?? 0}`,
      category: feed.category || '',
      subcategory: feed.subcategory ?? null,
      pricePerTon: price?.price_rubles_per_ton ?? feed.price_per_ton ?? null,
      provenance: price?.provenance ?? fallbackProvenance(feed),
      lastUpdated: price?.price_date ?? feed.price_updated_at ?? null,
      region: price?.region ?? feed.region ?? null,
    };
  });
}

function sourceBadgeClass(kind: string | null | undefined): string {
  switch (kind) {
    case 'direct':
      return 'bg-green-500/10 text-green-700';
    case 'benchmark':
      return 'bg-amber-500/10 text-amber-700';
    case 'manual':
      return 'bg-blue-500/10 text-blue-600';
    case 'seed':
      return 'bg-slate-500/10 text-slate-600';
    default:
      return 'bg-gray-500/10 text-gray-500';
  }
}

function sourceLabel(kind: string | null | undefined, t: (key: string, fallback?: string) => string): string {
  if (!kind) {
    return t('prices.noDash', '-');
  }

  return t(`prices.provenanceKinds.${kind}`, kind);
}

function sourceSecondaryLabel(
  provenance: PriceProvenance | null,
  t: (key: string, fallback?: string) => string,
): string | null {
  if (!provenance) {
    return null;
  }

  if (provenance.kind === 'direct') {
    return provenance.source_domain ?? t('prices.noPreciseSource', 'No precise source page available.');
  }

  if (provenance.kind === 'benchmark' && provenance.benchmark_level) {
    return t(`prices.benchmarkLevels.${provenance.benchmark_level}`, provenance.benchmark_level);
  }

  return null;
}

export function PricesPanel() {
  const { t, i18n } = useTranslationWithFallback();
  const queryClient = useQueryClient();
  const [prices, setPrices] = useState<PriceDialogRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [fetching, setFetching] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editValue, setEditValue] = useState('');
  const [sourceFilter, setSourceFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [detailsRow, setDetailsRow] = useState<PriceDialogRow | null>(null);

  useEffect(() => {
    void loadPrices();
  }, []);

  const loadPrices = async () => {
    setLoading(true);
    const generatedCatalog = await loadGeneratedFeedCatalog().catch(() => [] as Feed[]);

    try {
      const feedsResult = await feedsApi.list({ limit: 5000 });
      const feeds = feedsResult.data || [];
      const pricesResult = await pricesApi.list().catch(() => ({ data: [] as FeedPrice[] }));
      const rows = buildRows(feeds, pricesResult.data || []);
      setPrices(rows.length > 0 ? rows : buildRows(generatedCatalog, []));
    } catch (error) {
      console.error('Failed to load prices:', error);
      setPrices(buildRows(generatedCatalog, []));
    } finally {
      setLoading(false);
    }
  };

  const handleFetchPrices = async () => {
    setFetching(true);
    try {
      await feedsApi.sync();
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['feedCatalog'] }),
        queryClient.invalidateQueries({ queryKey: ['feedDetail'] }),
        queryClient.invalidateQueries({ queryKey: ['feedPrice'] }),
      ]);
      if (isNotificationEnabled('priceSync')) {
        toast.success(t('prices.fetchSuccess', 'Prices updated successfully'));
      }
      await loadPrices();
    } catch (error) {
      console.error('Failed to fetch prices:', error);
      if (isNotificationEnabled('priceSync')) {
        toast.error(t('prices.fetchError', 'Error fetching prices'));
      }
    } finally {
      setFetching(false);
    }
  };

  const handleStartEdit = (feedId: number, currentPrice: number | null) => {
    setEditingId(feedId);
    setEditValue(currentPrice?.toString() || '');
  };

  const handleSaveEdit = async (feedId: number) => {
    const price = Number.parseFloat(editValue);
    if (Number.isNaN(price) || price < 0) {
      return;
    }

    try {
      await pricesApi.update(feedId, price);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['feedCatalog'] }),
        queryClient.invalidateQueries({ queryKey: ['feedDetail'] }),
        queryClient.invalidateQueries({ queryKey: ['feedPrice'] }),
      ]);
      setPrices((previous) =>
        previous.map((row) =>
          row.feedId === feedId
            ? {
                ...row,
                pricePerTon: price,
                provenance: {
                  kind: 'manual',
                  is_precise_source: false,
                  anchor_sources: [],
                },
                lastUpdated: new Date().toISOString().slice(0, 10),
              }
            : row,
        ),
      );
      setDetailsRow((current) =>
        current && current.feedId === feedId
          ? {
              ...current,
              pricePerTon: price,
              provenance: {
                kind: 'manual',
                is_precise_source: false,
                anchor_sources: [],
              },
              lastUpdated: new Date().toISOString().slice(0, 10),
            }
          : current,
      );
      toast.success(t('prices.savePrice', 'Save price'));
    } catch (error) {
      console.error('Failed to save price:', error);
      toast.error(t('prices.fetchError', 'Error fetching prices'));
    }

    setEditingId(null);
    setEditValue('');
  };

  const filteredPrices = useMemo(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();

    return prices.filter((price) => {
      const provenanceKind = price.provenance?.kind ?? 'unknown';
      const matchesSource = sourceFilter === 'all' || provenanceKind === sourceFilter;
      if (!matchesSource) {
        return false;
      }

      if (!normalizedQuery) {
        return true;
      }

      const haystack = [
        price.feedName,
        price.category,
        price.subcategory ?? '',
        provenanceKind,
        sourceSecondaryLabel(price.provenance, t) ?? '',
      ]
        .join(' ')
        .toLowerCase();

      return haystack.includes(normalizedQuery);
    });
  }, [prices, searchQuery, sourceFilter, t]);

  const handleOpenSource = async (url: string) => {
    try {
      await openExternalUrl(url);
    } catch (error) {
      console.error('Failed to open source URL:', error);
      toast.error(t('prices.fetchError', 'Error fetching prices'));
    }
  };

  const categoryHeader = t('prices.category', 'Category');
  const syncLabel = t('prices.sync', 'Synchronize');
  const categoryLanguage = i18n.resolvedLanguage?.startsWith('en') ? 'en' : 'ru';
  const priceLocale = i18n.resolvedLanguage?.startsWith('en') ? 'en-US' : 'ru-RU';

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-hidden bg-[--bg-base]">
      <header className="flex items-center justify-between gap-3 border-b border-[--border] px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <Icon icon={DollarSign} size={20} className="text-[--accent]" />
          <h1 className="truncate text-sm font-medium text-[--text-primary]">{t('prices.title', 'Feed prices')}</h1>
          <span className="text-xs text-[--text-disabled]">({filteredPrices.length})</span>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Input
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder={t('feedLibrary.searchFeeds', 'Search feeds...')}
            className="h-8 w-56 text-xs"
          />

          <select
            value={sourceFilter}
            onChange={(event) => setSourceFilter(event.target.value)}
            className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-2 py-1.5 text-xs text-[--text-primary]"
          >
            <option value="all">{t('prices.allSources', 'All sources')}</option>
            <option value="direct">{sourceLabel('direct', t)}</option>
            <option value="benchmark">{sourceLabel('benchmark', t)}</option>
            <option value="manual">{sourceLabel('manual', t)}</option>
            <option value="seed">{sourceLabel('seed', t)}</option>
          </select>

          <Button variant="outline" size="sm" onClick={handleFetchPrices} disabled={fetching}>
            <Icon icon={fetching ? Loader2 : RefreshCw} size={14} className={cn('mr-1.5', fetching && 'animate-spin')} />
            {syncLabel}
          </Button>
        </div>
      </header>

      <div className="flex-1 overflow-auto">
        {loading ? (
          <div className="flex h-32 items-center justify-center">
            <Icon icon={Loader2} size={20} className="animate-spin text-[--text-disabled]" />
          </div>
        ) : filteredPrices.length === 0 ? (
          <div className="flex h-32 items-center justify-center text-xs text-[--text-disabled]">
            {t('prices.noData', 'No price data available')}
          </div>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b border-[--border] bg-[--bg-surface]">
                <th className="px-4 py-2 text-left font-medium text-[--text-secondary]">{t('prices.feedName', 'Feed')}</th>
                <th className="w-36 px-3 py-2 text-left font-medium text-[--text-secondary]">{categoryHeader}</th>
                <th className="w-32 px-3 py-2 text-right font-medium text-[--text-secondary]">{t('prices.pricePerTon', 'Price (RUB/t)')}</th>
                <th className="w-52 px-3 py-2 text-center font-medium text-[--text-secondary]">{t('prices.source', 'Source')}</th>
                <th className="w-28 px-3 py-2 text-center font-medium text-[--text-secondary]">{t('prices.lastUpdated', 'Last updated')}</th>
                <th className="w-32 px-3 py-2 text-center font-medium text-[--text-secondary]">{t('prices.actions', 'Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {filteredPrices.map((row) => {
                const categoryLabel = getFeedCategoryLabel(row.category, categoryLanguage);
                const provenanceKind = row.provenance?.kind ?? null;
                const secondaryLabel = sourceSecondaryLabel(row.provenance, t);
                const hasPrice = row.pricePerTon != null;

                return (
                  <tr key={row.feedId} className="border-b border-[--border] transition-colors hover:bg-[--bg-hover]">
                    <td className="px-4 py-2 font-medium text-[--text-primary]">{row.feedName}</td>
                    <td className="px-3 py-2 text-[--text-secondary]">{categoryLabel || t('prices.noDash', '-')}</td>
                    <td className="px-3 py-2 text-right">
                      {editingId === row.feedId ? (
                        <Input
                          type="number"
                          value={editValue}
                          onChange={(event) => setEditValue(event.target.value)}
                          onKeyDown={(event) => {
                            if (event.key === 'Enter') {
                              void handleSaveEdit(row.feedId);
                            }
                            if (event.key === 'Escape') {
                              setEditingId(null);
                              setEditValue('');
                            }
                          }}
                          className="w-24 text-right text-xs"
                          min={0}
                          autoFocus
                        />
                      ) : (
                        <span className={cn('font-mono', hasPrice ? 'text-[--text-primary]' : 'text-[--text-disabled]')}>
                          {hasPrice ? row.pricePerTon!.toLocaleString(priceLocale) : t('prices.noDash', '-')}
                        </span>
                      )}
                    </td>
                    <td className="px-3 py-2 text-center">
                      {provenanceKind ? (
                        <div className="flex flex-col items-center gap-1">
                          <span className={`inline-flex rounded px-1.5 py-0.5 text-[10px] ${sourceBadgeClass(provenanceKind)}`}>
                            {sourceLabel(provenanceKind, t)}
                          </span>
                          {secondaryLabel ? (
                            <span className="max-w-[180px] truncate text-[10px] text-[--text-secondary]">
                              {secondaryLabel}
                            </span>
                          ) : null}
                        </div>
                      ) : (
                        <span className="text-[--text-disabled]">{t('prices.noDash', '-')}</span>
                      )}
                    </td>
                    <td className="px-3 py-2 text-center text-[--text-disabled]">{row.lastUpdated || t('prices.noDash', '-')}</td>
                    <td className="px-3 py-2 text-center">
                      {editingId === row.feedId ? (
                        <div className="flex items-center justify-center gap-1">
                          <button
                            onClick={() => void handleSaveEdit(row.feedId)}
                            className="rounded p-1 text-green-600 hover:bg-green-500/10"
                            title={t('common.save', 'Save')}
                          >
                            <Icon icon={Check} size={14} />
                          </button>
                          <button
                            onClick={() => {
                              setEditingId(null);
                              setEditValue('');
                            }}
                            className="rounded p-1 text-red-500 hover:bg-red-500/10"
                            title={t('common.cancel', 'Cancel')}
                          >
                            <Icon icon={X} size={14} />
                          </button>
                        </div>
                      ) : (
                        <div className="flex items-center justify-center gap-2">
                          <button
                            onClick={() => setDetailsRow(row)}
                            className="rounded px-2 py-1 text-[11px] text-[--accent] hover:bg-[--bg-hover]"
                            title={t('prices.details', 'Details')}
                            disabled={!row.provenance}
                          >
                            {t('prices.details', 'Details')}
                          </button>
                          <button
                            onClick={() => handleStartEdit(row.feedId, row.pricePerTon)}
                            className="rounded p-1 text-[--text-disabled] hover:bg-[--bg-hover] hover:text-[--text-secondary]"
                            title={t('prices.editPrice', 'Edit price')}
                          >
                            <Icon icon={Edit2} size={14} />
                          </button>
                        </div>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <PriceProvenanceDialog
        row={detailsRow}
        open={detailsRow !== null}
        onClose={() => setDetailsRow(null)}
        onOpenSource={(url) => void handleOpenSource(url)}
      />
    </div>
  );
}
