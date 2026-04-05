import { useCallback, useMemo, useState } from 'react';
import { X, Search, Plus, Check, Warehouse } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Input } from '../ui/Input';
import { Button } from '../ui/Button';
import { useRationStore, calculateLocalNutrients } from '@/stores/rationStore';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getCategoryLabel } from '@/lib/feed-categories';
import { getFeedDisplayName, resolveFeedLanguage } from '@/lib/feed-display';
import { cn } from '@/lib/utils';
import type { Feed } from '@/types/feed';

/** Feed group tab definition. */
interface GroupTab {
  id: string;
  /** i18n key for the tab label (under feedCatalog.*) */
  labelKey: string;
  /** Feed category slugs that belong to this group */
  categories: string[];
}

const GROUP_TABS: GroupTab[] = [
  { id: 'roughage', labelKey: 'groupRoughage', categories: ['roughage', 'silage', 'green_forage'] },
  { id: 'succulent', labelKey: 'groupSucculent', categories: ['succulent'] },
  { id: 'concentrate', labelKey: 'groupConcentrate', categories: ['grain', 'concentrate', 'compound_feed'] },
  { id: 'protein', labelKey: 'groupProtein', categories: ['protein', 'oilseed_meal'] },
  { id: 'animal_origin', labelKey: 'groupAnimalOrigin', categories: ['animal_origin'] },
  { id: 'mineral', labelKey: 'groupMineral', categories: ['mineral'] },
  { id: 'premix', labelKey: 'groupPremix', categories: ['premix'] },
  { id: 'vitamin', labelKey: 'groupVitamin', categories: ['additive'] },
  { id: 'other', labelKey: 'groupOther', categories: ['oil_fat', 'byproduct', 'root_crops', 'other'] },
];

interface SelectedFeedEntry {
  feed: Feed;
  quantity: number;
}

interface FeedCatalogDialogProps {
  onClose: () => void;
}

export function FeedCatalogDialog({ onClose }: FeedCatalogDialogProps) {
  const { t, i18n } = useTranslationWithFallback();
  const { addFeed, localItems, setNutrients, animalProperties, animalGroupId, farmBucket, toggleFarmBucketFeed } = useRationStore();
  const feedLanguage = resolveFeedLanguage(i18n.resolvedLanguage);

  const [search, setSearch] = useState('');
  const [activeGroup, setActiveGroup] = useState<string | null>(null);
  const [selected, setSelected] = useState<Map<number, SelectedFeedEntry>>(new Map());

  const { feeds } = useFeedCatalog(search, {
    species: animalProperties.species,
    stageContext: animalGroupId ?? undefined,
  });

  /** Feeds filtered by the active group tab, selected sorted to top. */
  const filteredFeeds = useMemo(() => {
    let result = feeds;

    if (activeGroup) {
      const tab = GROUP_TABS.find((g) => g.id === activeGroup);
      if (tab) {
        result = feeds.filter((feed) => {
          const cat = feed.category ?? 'other';
          return tab.categories.includes(cat);
        });
      }
    }

    if (selected.size > 0) {
      return [...result].sort((a, b) => {
        const aSelected = selected.has(a.id) ? 0 : 1;
        const bSelected = selected.has(b.id) ? 0 : 1;
        return aSelected - bSelected;
      });
    }

    return result;
  }, [activeGroup, feeds, selected]);

  const toggleSelect = useCallback((feed: Feed) => {
    setSelected((prev) => {
      const next = new Map(prev);
      if (next.has(feed.id)) {
        next.delete(feed.id);
      } else {
        next.set(feed.id, { feed, quantity: 1.0 });
      }
      return next;
    });
  }, []);

  const updateQuantity = useCallback((feedId: number, quantity: number) => {
    setSelected((prev) => {
      const entry = prev.get(feedId);
      if (!entry) return prev;
      const next = new Map(prev);
      next.set(feedId, { ...entry, quantity: Math.max(0.01, quantity) });
      return next;
    });
  }, []);

  const handleAddSelected = useCallback(() => {
    const entries = Array.from(selected.values());
    if (entries.length === 0) return;

    let updatedItems = [...localItems.map((item) => ({
      id: item.id,
      feed: item.feed,
      amount_kg: item.amount_kg,
      is_locked: item.is_locked,
    }))];

    for (const entry of entries) {
      addFeed(entry.feed, entry.quantity);
      updatedItems.push({
        id: `temp-${Date.now()}-${entry.feed.id}`,
        feed: entry.feed,
        amount_kg: entry.quantity,
        is_locked: false,
      });
    }

    setNutrients(calculateLocalNutrients(updatedItems));
    onClose();
  }, [addFeed, localItems, onClose, selected, setNutrients]);

  /** Format a nutrient value compactly, locale-aware. */
  const locale = i18n.language?.startsWith('ru') ? 'ru-RU' : 'en-US';
  const compactVal = (v: number | undefined): string => {
    if (v === undefined || v === null || v <= 0) return '—';
    const decimals = v >= 100 ? 0 : v >= 10 ? 1 : 2;
    return v.toLocaleString(locale, {
      minimumFractionDigits: 0,
      maximumFractionDigits: decimals,
    });
  };

  /** Group counts for badge display on tabs. */
  const groupCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const tab of GROUP_TABS) {
      counts[tab.id] = feeds.filter((f) => tab.categories.includes(f.category ?? 'other')).length;
    }
    return counts;
  }, [feeds]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />

      {/* Dialog */}
      <div
        className="relative z-10 flex flex-col rounded-[--radius-md] border shadow-xl"
        style={{
          background: 'var(--bg-surface)',
          borderColor: 'var(--border)',
          width: 'min(960px, 90vw)',
          height: 'min(720px, 85vh)',
        }}
      >
        {/* Header */}
        <div className="flex items-start justify-between px-5 pt-4 pb-3 border-b border-[--border]">
          <div>
            <h2 className="text-base font-semibold text-[--text-primary]">
              {t('feedCatalog.browseTitle')}
            </h2>
            <p className="mt-0.5 text-xs text-[--text-secondary]">
              {t('feedCatalog.browseSubtitle')}
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-[--radius-sm] text-[--text-secondary] hover:bg-[--bg-hover] hover:text-[--text-primary]"
          >
            <Icon icon={X} size={18} />
          </button>
        </div>

        {/* Search + Tabs */}
        <div className="px-5 py-3 border-b border-[--border] space-y-3">
          {/* Search */}
          <div className="relative max-w-sm">
            <Icon
              icon={Search}
              size={14}
              className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[--text-disabled]"
            />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t('feedCatalog.searchPlaceholder')}
              className="pl-8 h-8 text-sm"
            />
          </div>

          {/* Tab pills */}
          <div className="flex flex-wrap gap-1.5">
            <button
              onClick={() => setActiveGroup(null)}
              className={cn(
                'px-2.5 py-1 rounded-full text-xs font-medium transition-colors',
                activeGroup === null
                  ? 'bg-[--accent] text-[--text-inverse]'
                  : 'bg-[--bg-hover] text-[--text-secondary] hover:text-[--text-primary]',
              )}
            >
              {t('feedCatalog.allGroups')}
              <span className="ml-1 opacity-70">({feeds.length})</span>
            </button>
            {GROUP_TABS.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveGroup(tab.id)}
                className={cn(
                  'px-2.5 py-1 rounded-full text-xs font-medium transition-colors',
                  activeGroup === tab.id
                    ? 'bg-[--accent] text-[--text-inverse]'
                    : 'bg-[--bg-hover] text-[--text-secondary] hover:text-[--text-primary]',
                )}
              >
                {t(`feedCatalog.${tab.labelKey}`)}
                <span className="ml-1 opacity-70">({groupCounts[tab.id] ?? 0})</span>
              </button>
            ))}
          </div>
        </div>

        {/* Selected feeds strip */}
        {selected.size > 0 && (
          <div className="px-5 py-2 border-b border-[--border] bg-[--bg-base]">
            <div className="text-[10px] text-[--text-secondary] mb-1.5">
              {t('feedCatalog.selectedCount', { count: selected.size })}
            </div>
            <div className="flex gap-1.5 overflow-x-auto pb-1">
              {Array.from(selected.values()).map((entry) => (
                <div
                  key={entry.feed.id}
                  className="flex items-center gap-1.5 shrink-0 rounded-full border border-[--accent] bg-[--accent]/5 px-2.5 py-1 text-xs text-[--text-primary]"
                >
                  <span className="max-w-[160px] truncate">
                    {getFeedDisplayName(entry.feed, feedLanguage)}
                  </span>
                  <span className="text-[10px] text-[--text-secondary]">
                    {entry.quantity} {t('units.kg')}
                  </span>
                  <button
                    onClick={() => toggleSelect(entry.feed)}
                    className="ml-0.5 rounded-full p-0.5 text-[--text-disabled] hover:text-[--text-primary] hover:bg-[--bg-hover]"
                  >
                    <Icon icon={X} size={10} />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Feed grid */}
        <div className="flex-1 overflow-y-auto px-5 py-3">
          {filteredFeeds.length === 0 ? (
            <div className="flex items-center justify-center h-full">
              <p className="text-sm text-[--text-secondary]">
                {t('feedCatalog.noFeedsInGroup')}
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
              {filteredFeeds.map((feed) => {
                const isSelected = selected.has(feed.id);
                const entry = selected.get(feed.id);
                const categoryLabel = getCategoryLabel(
                  feed.category ?? 'other',
                  feedLanguage === 'en' ? 'en' : 'ru',
                );

                return (
                  <div
                    key={feed.id}
                    className={cn(
                      'rounded-[--radius-md] border p-3 transition-colors cursor-pointer',
                      isSelected
                        ? 'border-[--accent] bg-[--accent]/5'
                        : 'border-[--border] hover:border-[--text-disabled] bg-[--bg-base]',
                    )}
                    onClick={() => toggleSelect(feed)}
                  >
                    <div className="flex items-start gap-2">
                      {/* Checkbox indicator */}
                      <div
                        className={cn(
                          'mt-0.5 w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors',
                          isSelected
                            ? 'bg-[--accent] border-[--accent]'
                            : 'border-[--border] bg-[--bg-base]',
                        )}
                      >
                        {isSelected && <Icon icon={Check} size={12} className="text-[--text-inverse]" />}
                      </div>

                      <div className="min-w-0 flex-1">
                        <div className="flex items-start justify-between gap-1">
                          <p className="text-sm font-medium text-[--text-primary] truncate">
                            {getFeedDisplayName(feed, feedLanguage)}
                          </p>
                          <button
                            onClick={(e) => { e.stopPropagation(); toggleFarmBucketFeed(feed.id); }}
                            className={cn(
                              'p-0.5 rounded shrink-0 transition-colors',
                              farmBucket.has(feed.id)
                                ? 'text-[--accent]'
                                : 'text-[--text-disabled] hover:text-[--accent]',
                            )}
                            title={farmBucket.has(feed.id) ? t('feedLibrary.removeFromFarm') : t('feedLibrary.addToFarm')}
                          >
                            <Icon icon={Warehouse} size={12} />
                          </button>
                        </div>
                        <p className="text-[10px] text-[--text-disabled] mt-0.5">
                          {categoryLabel}
                        </p>

                        {/* Nutrient badges */}
                        <div className="mt-1.5 flex flex-wrap gap-1 text-[10px]">
                          {feed.dry_matter !== undefined && feed.dry_matter > 0 && (
                            <span className="rounded-full border border-[--border] bg-[--bg-surface] px-1.5 py-0.5 text-[--text-disabled]">
                              {t('nutrients.abbr.dry_matter')} {compactVal(feed.dry_matter)}%
                            </span>
                          )}
                          {feed.crude_protein !== undefined && feed.crude_protein > 0 && (
                            <span className="rounded-full border border-[--border] bg-[--bg-surface] px-1.5 py-0.5 text-[--text-disabled]">
                              {t('nutrients.abbr.crude_protein')} {compactVal(feed.crude_protein)}
                            </span>
                          )}
                          {feed.energy_oe_cattle !== undefined && feed.energy_oe_cattle > 0 && (
                            <span className="rounded-full border border-[--border] bg-[--bg-surface] px-1.5 py-0.5 text-[--text-disabled]">
                              {t('nutrients.abbr.energy_oe_cattle')} {compactVal(feed.energy_oe_cattle)}
                            </span>
                          )}
                        </div>
                      </div>
                    </div>

                    {/* Quantity input — shown when selected */}
                    {isSelected && entry && (
                      <div
                        className="mt-2 flex items-center gap-2"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <label className="text-[10px] text-[--text-secondary] shrink-0">
                          {t('feedCatalog.quantity')}
                        </label>
                        <Input
                          type="number"
                          min={0.01}
                          step={0.5}
                          value={entry.quantity}
                          onChange={(e) => updateQuantity(feed.id, parseFloat(e.target.value) || 0.01)}
                          className="h-6 w-20 text-xs px-1.5"
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Sticky footer */}
        <div className="px-5 py-3 border-t border-[--border] flex items-center justify-between">
          <span className="text-xs text-[--text-secondary]">
            {t('feedCatalog.selectedCount', { count: selected.size })}
          </span>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={onClose}>
              {t('common.cancel')}
            </Button>
            <Button
              size="sm"
              disabled={selected.size === 0}
              onClick={handleAddSelected}
            >
              <Icon icon={Plus} size={14} />
              {t('feedCatalog.addSelected')}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
