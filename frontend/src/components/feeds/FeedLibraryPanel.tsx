import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Search, ChevronRight, ChevronDown, Plus, RefreshCw, Database, AlertTriangle, CheckCircle2, GripVertical, Loader2, Warehouse, CheckSquare, Square, X } from 'lucide-react';
import toast from 'react-hot-toast';
import { Icon } from '../ui/Icon';
import { Input } from '../ui/Input';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { FeedDetailModal } from './FeedDetailModal';
import { CreateFeedModal } from './CreateFeedModal';
import { useRationStore } from '@/stores/rationStore';
import { feedsApi } from '@/lib/api';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getFeedCategoryLabel } from '@/lib/feed-categories';
import { getFeedDisplayName, resolveFeedLanguage } from '@/lib/feed-display';
import { feedSuitabilityBadgeVariant } from '@/lib/feed-suitability';
import type { Feed } from '@/types/feed';

export function FeedLibraryPanel() {
  const { t, i18n } = useTranslationWithFallback();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState('');
  const [expanded, setExpanded] = useState<Set<string>>(new Set(['concentrate', 'silage']));
  const [selectedFeed, setSelectedFeed] = useState<Feed | null>(null);
  const [showCreateFeed, setShowCreateFeed] = useState(false);
  const [recoveryTriggered, setRecoveryTriggered] = useState(false);
  const [showAllFeeds, setShowAllFeeds] = useState(false);
  const [selectedFeedIds, setSelectedFeedIds] = useState<Set<number>>(new Set());

  const {
    addFeed,
    animalGroupId,
    animalProperties,
    farmBucket,
    farmBucketActive,
    setFarmBucketActive,
    toggleFarmBucketFeed,
    addToFarmBucket,
    removeFromFarmBucket,
  } = useRationStore();
  const categoryLanguage = resolveFeedLanguage(i18n.resolvedLanguage);
  const {
    feeds,
    total,
    isLoading,
    error,
    liveData,
    usingFallback,
    contextAware,
  } = useFeedCatalog(search, {
    species: animalProperties.species,
    stageContext: animalGroupId ?? undefined,
  });

  const syncMutation = useMutation({
    mutationFn: feedsApi.sync,
    onSuccess: async (result) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['feedCatalog'] }),
        queryClient.invalidateQueries({ queryKey: ['feedDetail'] }),
        queryClient.invalidateQueries({ queryKey: ['feedPrice'] }),
      ]);
      toast.success(
        t('feedLibrary.syncSuccess', {
          feeds: result.data.feeds_total,
          prices: result.data.prices_updated,
        }),
      );
    },
    onError: (mutationError) => {
      toast.error(
        mutationError instanceof Error && mutationError.message
          ? mutationError.message
          : t('feedLibrary.syncError'),
      );
    },
  });

  useEffect(() => {
    if (search || recoveryTriggered || error || syncMutation.isPending || usingFallback || !liveData) {
      return;
    }

    if ((liveData.total ?? 0) === 0 && (liveData.data?.length ?? 0) === 0) {
      setRecoveryTriggered(true);
      syncMutation.mutate();
    }
  }, [error, liveData, recoveryTriggered, search, syncMutation, usingFallback]);

  const visibleFeeds = useMemo(() => {
    let result = contextAware && !showAllFeeds
      ? feeds.filter((feed) => feed.suitability_status !== 'restricted')
      : feeds;
    if (farmBucketActive && farmBucket.size > 0) {
      result = result.filter((feed) => farmBucket.has(feed.id));
    }
    return result;
  }, [contextAware, farmBucket, farmBucketActive, feeds, showAllFeeds]);

  const hiddenRestrictedCount = useMemo(
    () => (contextAware && !showAllFeeds
      ? feeds.filter((feed) => feed.suitability_status === 'restricted').length
      : 0),
    [contextAware, feeds, showAllFeeds],
  );

  const categories = useMemo(() => {
    const grouped = visibleFeeds.reduce((acc, feed) => {
      const category = feed.category ?? 'other';
      if (!acc[category]) acc[category] = [];
      acc[category].push(feed);
      return acc;
    }, {} as Record<string, Feed[]>);

    return Object.entries(grouped).map(([id, feeds]) => ({
      id,
      name: getFeedCategoryLabel(id, categoryLanguage),
      feeds,
    }));
  }, [categoryLanguage, visibleFeeds]);

  useEffect(() => {
    const visibleIds = new Set(visibleFeeds.map((feed) => feed.id));
    setSelectedFeedIds((previous) => {
      const next = new Set([...previous].filter((feedId) => visibleIds.has(feedId)));
      return next.size === previous.size ? previous : next;
    });
  }, [visibleFeeds]);

  const selectedFeeds = useMemo(
    () => visibleFeeds.filter((feed) => selectedFeedIds.has(feed.id)),
    [selectedFeedIds, visibleFeeds],
  );
  const selectedCount = selectedFeeds.length;
  const selectedVisibleFeedIds = useMemo(
    () => selectedFeeds.map((feed) => feed.id),
    [selectedFeeds],
  );
  const allVisibleSelected = visibleFeeds.length > 0 && selectedCount === visibleFeeds.length;

  const toggleExpand = (id: string) => {
    const next = new Set(expanded);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setExpanded(next);
  };

  const handleAddFeed = (feed: Feed) => {
    if (feed.suitability_status === 'restricted') {
      toast.error(t('feedLibrary.restrictedAddError'));
      return;
    }

    addFeed(feed, 1.0);
  };

  const toggleSelectedFeed = (feedId: number) => {
    setSelectedFeedIds((previous) => {
      const next = new Set(previous);
      if (next.has(feedId)) {
        next.delete(feedId);
      } else {
        next.add(feedId);
      }
      return next;
    });
  };

  const handleSelectVisible = () => {
    setSelectedFeedIds(new Set(visibleFeeds.map((feed) => feed.id)));
  };

  const handleClearSelection = () => {
    setSelectedFeedIds(new Set());
  };

  const handleAddSelectedToRation = () => {
    const eligibleFeeds = selectedFeeds.filter((feed) => feed.suitability_status !== 'restricted');
    const skippedCount = selectedCount - eligibleFeeds.length;

    if (eligibleFeeds.length === 0) {
      toast.error(t('feedLibrary.restrictedAddError'));
      return;
    }

    for (const feed of eligibleFeeds) {
      addFeed(feed, 1.0);
    }

    toast.success(t('feedLibrary.batchAddToRationSuccess', { count: eligibleFeeds.length }));
    if (skippedCount > 0) {
      toast(t('feedLibrary.batchRestrictedSkipped', { count: skippedCount }));
    }
    handleClearSelection();
  };

  const handleAddSelectedToFarm = () => {
    if (selectedVisibleFeedIds.length === 0) {
      return;
    }

    addToFarmBucket(selectedVisibleFeedIds);
    toast.success(t('feedLibrary.batchAddToFarmSuccess', { count: selectedVisibleFeedIds.length }));
    handleClearSelection();
  };

  const handleRemoveSelectedFromFarm = () => {
    const removableFeedIds = selectedVisibleFeedIds.filter((feedId) => farmBucket.has(feedId));
    if (removableFeedIds.length === 0) {
      return;
    }

    removeFromFarmBucket(removableFeedIds);
    toast.success(t('feedLibrary.batchRemoveFromFarmSuccess', { count: removableFeedIds.length }));
    handleClearSelection();
  };

  return (
    <aside
      className="w-72 flex-shrink-0 border-l flex flex-col overflow-hidden"
      style={{
        background: 'var(--bg-sidebar)',
        borderColor: 'var(--border)',
      }}
    >
      <div className="px-3 py-2 border-b border-[--border]">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-medium text-[--text-primary]">{t('feedLibrary.title')}</h3>
          <span className="text-[10px] text-[--text-disabled]">{visibleFeeds.length} {t('settings.entries')}</span>
        </div>
      </div>

      <div className="p-2 border-b border-[--border]">
        <div className="relative">
          <Icon icon={Search} size={14} className="absolute left-2 top-1/2 -translate-y-1/2 text-[--text-disabled]" />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t('feedLibrary.searchFeeds')}
            className="pl-7 h-7 text-xs"
          />
        </div>
        {contextAware ? (
          <label className="mt-2 flex items-center gap-2 text-[10px] text-[--text-secondary]">
            <input
              type="checkbox"
              checked={showAllFeeds}
              onChange={(event) => setShowAllFeeds(event.target.checked)}
              className="h-3.5 w-3.5 rounded border border-[--border] bg-[--bg-base] accent-[--accent]"
            />
            <span>{t('feedLibrary.showAllFeeds')}</span>
            {!showAllFeeds && hiddenRestrictedCount > 0 ? (
              <span className="text-[--text-disabled]">
                {t('feedLibrary.hiddenRestrictedCount', { count: hiddenRestrictedCount })}
              </span>
            ) : null}
          </label>
        ) : null}
        {farmBucket.size > 0 && (
          <label className="mt-1.5 flex items-center gap-2 text-[10px] text-[--text-secondary]">
            <input
              type="checkbox"
              checked={farmBucketActive}
              onChange={(event) => setFarmBucketActive(event.target.checked)}
              className="h-3.5 w-3.5 rounded border border-[--border] bg-[--bg-base] accent-[--accent]"
            />
            <Icon icon={Warehouse} size={12} className="text-[--text-disabled]" />
            <span>{t('feedLibrary.myFeedsOnly')}</span>
            <span className="text-[--text-disabled]">({farmBucket.size})</span>
          </label>
        )}
        <div className="mt-2 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-2">
          <div className="flex items-center justify-between gap-2">
            <div className="text-[10px] font-medium text-[--text-secondary]">
              {t('feedLibrary.selectedCount', { count: selectedCount })}
            </div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={allVisibleSelected ? handleClearSelection : handleSelectVisible}
                className="rounded p-1 text-[--text-disabled] transition-colors hover:bg-[--bg-hover] hover:text-[--text-primary]"
                title={allVisibleSelected ? t('feedLibrary.clearSelection') : t('feedLibrary.selectVisible')}
              >
                <Icon icon={allVisibleSelected ? CheckSquare : Square} size={12} />
              </button>
              <button
                type="button"
                onClick={handleClearSelection}
                className="rounded p-1 text-[--text-disabled] transition-colors hover:bg-[--bg-hover] hover:text-[--text-primary] disabled:opacity-40"
                title={t('feedLibrary.clearSelection')}
                disabled={selectedCount === 0}
              >
                <Icon icon={X} size={12} />
              </button>
            </div>
          </div>
          <div className="mt-2 grid gap-1">
            <Button
              variant="outline"
              size="sm"
              className="h-7 justify-start text-[10px]"
              onClick={handleAddSelectedToRation}
              disabled={selectedCount === 0}
            >
              <Icon icon={Plus} size={12} />
              {t('feedLibrary.addSelectedToRation')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-7 justify-start text-[10px]"
              onClick={handleAddSelectedToFarm}
              disabled={selectedCount === 0}
            >
              <Icon icon={Warehouse} size={12} />
              {t('feedLibrary.addSelectedToFarm')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-7 justify-start text-[10px]"
              onClick={handleRemoveSelectedFromFarm}
              disabled={!selectedFeeds.some((feed) => farmBucket.has(feed.id))}
            >
              <Icon icon={Warehouse} size={12} />
              {t('feedLibrary.removeSelectedFromFarm')}
            </Button>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {isLoading || syncMutation.isPending ? (
          <div className="flex items-center justify-center py-8 text-[--text-secondary]">
            <Icon icon={Loader2} size={16} className="animate-spin mr-2" />
            <span className="text-xs">{syncMutation.isPending ? t('feedLibrary.syncFromCapRu') : t('common.loading')}</span>
          </div>
        ) : categories.length === 0 ? (
          <div className="text-center py-8 text-xs text-[--text-secondary]">
            {hiddenRestrictedCount > 0 ? t('feedLibrary.noVisibleFeeds') : t('feedLibrary.noFeedsFound')}
          </div>
        ) : (
          categories.map((category) => {
            const isExpanded = expanded.has(category.id);
            if (category.feeds.length === 0) return null;

            return (
              <div key={category.id} className="mb-1">
                <button
                  onClick={() => toggleExpand(category.id)}
                  className="w-full flex items-center gap-2 px-2 py-1.5 text-xs hover:bg-[--bg-hover] rounded-[--radius-sm]"
                >
                  <Icon icon={isExpanded ? ChevronDown : ChevronRight} size={14} className="text-[--text-disabled]" />
                  <span className="flex-1 text-left text-[--text-primary]">{category.name}</span>
                  <span className="text-[10px] text-[--text-disabled]">({category.feeds.length})</span>
                </button>
                {isExpanded && (
                  <div className="ml-2">
                    {category.feeds.map((feed) => (
                      <div
                        key={feed.id}
                        className={`flex items-center gap-1 px-2 py-1.5 text-xs rounded-[--radius-sm] group ${
                          feed.suitability_status === 'restricted'
                            ? 'opacity-60'
                            : 'hover:bg-[--bg-hover]'
                        }`}
                      >
                        <input
                          type="checkbox"
                          checked={selectedFeedIds.has(feed.id)}
                          onChange={() => toggleSelectedFeed(feed.id)}
                          className="h-3.5 w-3.5 rounded border border-[--border] bg-[--bg-base] accent-[--accent]"
                          aria-label={t('feedLibrary.selectFeed')}
                        />
                        <Icon
                          icon={GripVertical}
                          size={12}
                          className="text-[--text-disabled] cursor-grab opacity-0 group-hover:opacity-100 transition-opacity"
                        />
                        <button
                          onClick={() => setSelectedFeed(feed)}
                          className="min-w-0 flex-1 text-left truncate text-[--text-secondary] hover:text-[--text-primary]"
                        >
                          {getFeedDisplayName(feed, categoryLanguage)}
                        </button>
                        {contextAware && feed.suitability_status && feed.suitability_status !== 'appropriate' ? (
                          <Badge
                            variant={feedSuitabilityBadgeVariant(feed.suitability_status)}
                            className="shrink-0"
                          >
                            {t(`feedLibrary.suitability.${feed.suitability_status}`)}
                          </Badge>
                        ) : null}
                        <button
                          onClick={() => toggleFarmBucketFeed(feed.id)}
                          className={`p-0.5 rounded transition-opacity ${
                            farmBucket.has(feed.id)
                              ? 'text-[--accent] opacity-100'
                              : 'text-[--text-disabled] hover:text-[--accent] opacity-0 group-hover:opacity-100'
                          }`}
                          title={farmBucket.has(feed.id) ? t('feedLibrary.removeFromFarm') : t('feedLibrary.addToFarm')}
                        >
                          <Icon icon={Warehouse} size={10} />
                        </button>
                        <button
                          onClick={() => handleAddFeed(feed)}
                          className="p-0.5 rounded text-[--text-disabled] hover:text-[--accent] hover:bg-[--bg-active] opacity-0 group-hover:opacity-100 transition-opacity disabled:opacity-40 disabled:hover:text-[--text-disabled] disabled:hover:bg-transparent"
                          title={
                            feed.suitability_status === 'restricted'
                              ? t('feedLibrary.restrictedAddTitle')
                              : t('feedLibrary.addToRation')
                          }
                          disabled={feed.suitability_status === 'restricted'}
                        >
                          <Icon icon={Plus} size={12} />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      <div className="p-2 border-t border-[--border] space-y-1">
        <Button variant="outline" size="sm" className="w-full justify-start" onClick={() => setShowCreateFeed(true)}>
          <Icon icon={Plus} size={14} />
          {t('feedLibrary.createFeed')}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="w-full justify-start"
          onClick={() => syncMutation.mutate()}
          disabled={syncMutation.isPending}
        >
          <Icon icon={RefreshCw} size={14} className={syncMutation.isPending ? 'animate-spin' : ''} />
          {syncMutation.isPending ? t('common.loading') : t('feedLibrary.syncFromCapRu')}
        </Button>
      </div>

      <div className="p-2 border-t border-[--border]">
        <div className="bg-[--bg-surface] border border-[--border] rounded-[--radius-md] p-2">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-1.5">
              <Icon icon={Database} size={14} className="text-[--accent]" />
              <span className="text-xs font-medium text-[--text-primary]">{t('feedLibrary.libraryStatus')}</span>
            </div>
            <div
              className={`flex items-center gap-1 text-[10px] ${
                usingFallback ? 'text-[--status-warn]' : 'text-[--status-ok]'
              }`}
            >
              <Icon icon={usingFallback ? AlertTriangle : CheckCircle2} size={12} />
              <span>{usingFallback ? t('feedLibrary.fallbackMode') : t('feedLibrary.liveMode')}</span>
            </div>
          </div>
          <p className="text-[10px] text-[--text-secondary]">
            {usingFallback ? t('feedLibrary.catalogFallbackNote') : t('feedLibrary.catalogLiveNote')}
          </p>
          <p className="mt-2 text-[10px] text-[--text-secondary]">
            {t('feedLibrary.catalogEntriesNote', { count: total })}
          </p>
          <p className="mt-2 text-[10px] text-[--text-secondary]">
            {usingFallback
              ? t('feedLibrary.suitabilityFallbackNote')
              : t('feedLibrary.suitabilityLiveNote')}
          </p>
        </div>
      </div>

      {selectedFeed && <FeedDetailModal feed={selectedFeed} onClose={() => setSelectedFeed(null)} />}

      {showCreateFeed && (
        <CreateFeedModal
          onSave={(feed) => {
            handleAddFeed(feed);
            queryClient.invalidateQueries({ queryKey: ['feedCatalog'] });
          }}
          onClose={() => setShowCreateFeed(false)}
        />
      )}
    </aside>
  );
}
