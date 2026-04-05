import { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { X, Plus, Edit2 } from 'lucide-react';

import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { useRationStore, calculateLocalNutrients } from '@/stores/rationStore';
import { feedsApi } from '@/lib/api';
import { loadGeneratedFeedDetails } from '@/lib/feed-catalog';
import { getFeedCategoryLabel, getFeedSubcategoryLabel } from '@/lib/feed-categories';
import { getFeedDetailSections, getFeedPriceUnit } from '@/lib/feed-detail-registry';
import {
  getFeedDisplayName,
  getFeedProfileAudit,
  getFeedSecondaryName,
  resolveFeedLanguage,
} from '@/lib/feed-display';
import {
  criticalCoverageBadgeVariant,
  localizeCriticalNutrientKey,
} from '@/lib/feed-critical-audit';
import {
  feedSuitabilityBadgeVariant,
  hasContextualSuitability,
  localizeFeedSuitabilityNote,
} from '@/lib/feed-suitability';
import type { Feed } from '@/types/feed';

interface FeedDetailModalProps {
  feed: Feed;
  onClose: () => void;
}

function normalizeFeedName(value: string | null | undefined): string {
  return (value ?? '')
    .trim()
    .toLowerCase()
    .replace(/\u0451/g, '\u0435');
}

function refersToSameFeed(left: Feed, right: Feed): boolean {
  if (left.source_id && right.source_id && left.source_id === right.source_id) {
    return true;
  }

  const leftNames = new Set([
    normalizeFeedName(left.name_ru),
    normalizeFeedName(left.name_en),
  ]);
  const rightNames = [
    normalizeFeedName(right.name_ru),
    normalizeFeedName(right.name_en),
  ];

  const namesOverlap = rightNames.some((name) => name && leftNames.has(name));
  if (left.id === right.id && namesOverlap) {
    return true;
  }

  return namesOverlap;
}

export function FeedDetailModal({ feed, onClose }: FeedDetailModalProps) {
  const { t, i18n } = useTranslation();
  const { addFeed, localItems, setLocalItems, setNutrients, animalGroupId, animalProperties } = useRationStore();

  const detailQuery = useQuery({
    queryKey: ['feedDetail', feed.id, animalProperties.species, animalGroupId],
    queryFn: async () => {
      try {
        const response = await feedsApi.get(feed.id, {
          species: animalProperties.species,
          stageContext: animalGroupId ?? undefined,
        });
        return response.data;
      } catch {
        const details = await loadGeneratedFeedDetails();
        return details.get(feed.id) ?? feed;
      }
    },
    initialData: feed,
    staleTime: Infinity,
  });

  const detailFeed = detailQuery.data ?? feed;
  const language = resolveFeedLanguage(i18n.resolvedLanguage);
  
  const priceQuery = useQuery({
    queryKey: ['feedPrice', feed.id, language],
    queryFn: async () => {
      const response = await feedsApi.fetchFeedPrice(feed.id, language === 'ru' ? 'RU' : 'EN');
      return response.data;
    },
    staleTime: 1000 * 60 * 5,
    retry: false,
  });

  const activeFeed: Feed = priceQuery.data
    ? {
        ...detailFeed,
        price_per_ton: priceQuery.data.price,
        price_updated_at: priceQuery.data.price_date ?? detailFeed.price_updated_at,
        region: priceQuery.data.region ?? detailFeed.region,
      }
    : detailFeed;
  const profileAudit = getFeedProfileAudit(activeFeed);
  const displayName = getFeedDisplayName(activeFeed, language);
  const secondaryName = getFeedSecondaryName(activeFeed, language);
  const subcategoryLabel = getFeedSubcategoryLabel(activeFeed.subcategory, language);
  const detailSections = getFeedDetailSections(activeFeed, language);
  const priceUnit = getFeedPriceUnit(language);
  const priceLocale = language === 'en' ? 'en-US' : 'ru-RU';
  const resolvedPricePerTon = priceQuery.data?.price ?? activeFeed.price_per_ton;
  const feedForRation: Feed = resolvedPricePerTon == null
    ? activeFeed
    : {
        ...activeFeed,
        price_per_ton: resolvedPricePerTon,
        price_updated_at: priceQuery.data?.price_date ?? activeFeed.price_updated_at,
        region: priceQuery.data?.region ?? activeFeed.region,
      };
  const suitabilityNotes = (activeFeed.suitability_notes ?? []).map((note) =>
    localizeFeedSuitabilityNote(note, t),
  );
  const hasSuitability = hasContextualSuitability(activeFeed);
  const suitabilityVariant = feedSuitabilityBadgeVariant(activeFeed.suitability_status);
  const criticalAudit = activeFeed.critical_nutrient_audit;
  const criticalVariant = criticalCoverageBadgeVariant(criticalAudit?.coverage_status);
  const criticalMissingLabels = (criticalAudit?.missing_keys ?? []).map((key) =>
    localizeCriticalNutrientKey(key, t, i18n.resolvedLanguage),
  );

  const sectionLabels = {
    energy: language === 'ru' ? 'Энергия' : 'Energy',
    protein: language === 'ru' ? 'Протеин и аминокислоты' : 'Protein and amino acids',
    fiber: language === 'ru' ? 'Структура и углеводы' : 'Fiber and carbohydrates',
    minerals: language === 'ru' ? 'Минералы' : 'Minerals',
    vitamins: language === 'ru' ? 'Витамины и кофакторы' : 'Vitamins and cofactors',
  } as const;

  const sectionVariant = {
    present: 'success',
    partial: 'warning',
    missing: 'error',
  } as const;

  const handleAddToRation = () => {
    addFeed(feedForRation, 1.0);
    const updatedItems = [
      ...localItems,
      { id: `temp-${Date.now()}`, feed: feedForRation, amount_kg: 1.0, is_locked: false },
    ];
    setNutrients(calculateLocalNutrients(updatedItems));
    onClose();
  };

  useEffect(() => {
    if (resolvedPricePerTon == null) {
      return;
    }

    const nextItems = localItems.map((item) => {
      if (
        !refersToSameFeed(item.feed, activeFeed) ||
        item.feed.price_per_ton === resolvedPricePerTon
      ) {
        return item;
      }

      return {
        ...item,
        feed: {
          ...item.feed,
          ...activeFeed,
          price_per_ton: resolvedPricePerTon,
          price_updated_at: priceQuery.data?.price_date ?? item.feed.price_updated_at,
          region: priceQuery.data?.region ?? item.feed.region,
        },
      };
    });

    const changed = nextItems.some((item, index) => item !== localItems[index]);
    if (changed) {
      setLocalItems(nextItems);
    }
  }, [
    activeFeed.id,
    localItems,
    priceQuery.data?.price_date,
    priceQuery.data?.region,
    resolvedPricePerTon,
    setLocalItems,
  ]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      <div className="relative max-h-[80vh] w-full max-w-2xl overflow-hidden rounded-[--radius-lg] border border-[--border] bg-[--bg-base] shadow-xl">
        <div className="flex items-center justify-between border-b border-[--border] px-4 py-3">
          <div>
            <h2 className="text-sm font-medium text-[--text-primary]">{displayName}</h2>
            {secondaryName ? (
              <p className="text-xs text-[--text-secondary]">{secondaryName}</p>
            ) : null}
          </div>

          <div className="flex items-center gap-2">
            <Badge variant="secondary">
              {getFeedCategoryLabel(activeFeed.category ?? 'other', language)}
            </Badge>
            {subcategoryLabel ? (
              <Badge variant="default">{subcategoryLabel}</Badge>
            ) : null}
            <button
              onClick={onClose}
              className="rounded p-1 text-[--text-secondary] transition-colors hover:bg-[--bg-hover]"
            >
              <Icon icon={X} size={16} />
            </button>
          </div>
        </div>

        <div className="max-h-[60vh] overflow-y-auto p-4">
          <div className="mb-6 grid grid-cols-2 gap-4">
            <InfoCard label={t('nutrients.dmKg')} value={`${activeFeed.dry_matter ?? 0}%`} />
            <div className="rounded-[--radius-md] bg-[--bg-surface] p-3">
              <div className="mb-1 text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('prices.pricePerTon')}
              </div>
              <div className="text-sm font-medium text-[--text-primary]">
                {priceQuery.data
                  ? `${priceQuery.data.price.toLocaleString(priceLocale)} ${priceQuery.data.currency}`
                  : activeFeed.price_per_ton
                    ? `${activeFeed.price_per_ton.toLocaleString(priceLocale)} ${priceUnit}`
                    : priceQuery.isLoading
                      ? (language === 'ru' ? 'Синхронизация...' : 'Syncing...')
                      : '—'}
              </div>
              {priceQuery.data?.source_url ? (
                <a 
                  href={priceQuery.data.source_url} 
                  target="_blank" 
                  rel="noreferrer" 
                  className="mt-1 block text-[10px] text-blue-500 hover:underline"
                >
                  {language === 'ru' ? 'Источник' : 'Source'}
                </a>
              ) : null}
            </div>
          </div>

          <div className="mb-6 grid gap-3 md:grid-cols-4">
            <InfoCard
              label={t('feedLibrary.sourceTitle')}
              value={t(`feedLibrary.source.${profileAudit.sourceKey}`)}
            />
            <InfoCard
              label={t('feedLibrary.translationTitle')}
              value={t(`feedLibrary.translation.${profileAudit.translationKey}`)}
            />
            <InfoCard
              label={t('feedLibrary.profileTitle')}
              value={t(`feedLibrary.profile.${profileAudit.overallStatus}`)}
            />
            {subcategoryLabel ? (
              <InfoCard
                label={language === 'ru' ? 'Подкатегория' : 'Subcategory'}
                value={subcategoryLabel}
              />
            ) : null}
          </div>

          {hasSuitability ? (
            <div className="mb-6 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <div className="flex flex-wrap items-center gap-2">
                <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                  {t('feedLibrary.suitabilityTitle')}
                </div>
                {activeFeed.suitability_status ? (
                  <Badge variant={suitabilityVariant}>
                    {t(`feedLibrary.suitability.${activeFeed.suitability_status}`)}
                  </Badge>
                ) : null}
              </div>
              <p className="mt-2 text-xs text-[--text-secondary]">
                {t('feedLibrary.suitabilityContext')}
              </p>
              {(suitabilityNotes.length || activeFeed.suitability_max_inclusion_pct) ? (
                <ul className="mt-3 space-y-1 text-xs text-[--text-secondary]">
                  {suitabilityNotes.map((note) => (
                    <li key={note} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-1.5">
                      {note}
                    </li>
                  ))}
                  {activeFeed.suitability_max_inclusion_pct ? (
                    <li className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-1.5">
                      {t('feedLibrary.suitabilityMaxInclusion', {
                        value: activeFeed.suitability_max_inclusion_pct.toLocaleString(priceLocale, {
                          maximumFractionDigits: 1,
                        }),
                      })}
                    </li>
                  ) : null}
                </ul>
              ) : null}
            </div>
          ) : null}

          {criticalAudit ? (
            <div className="mb-6 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
              <div className="flex flex-wrap items-center gap-2">
                <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                  {t('feedLibrary.criticalTitle')}
                </div>
                <Badge variant={criticalVariant}>
                  {t(`feedLibrary.profile.${criticalAudit.coverage_status}`)}
                </Badge>
              </div>
              {criticalMissingLabels.length ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {criticalMissingLabels.map((label) => (
                    <Badge key={label} variant="warning">
                      {label}
                    </Badge>
                  ))}
                </div>
              ) : (
                <p className="mt-2 text-xs text-[--text-secondary]">
                  {t('feedLibrary.criticalNoGaps')}
                </p>
              )}
              <div className="mt-3 flex flex-wrap gap-2">
                {(criticalAudit.required_keys ?? []).map((key) => {
                  const label = localizeCriticalNutrientKey(key, t, i18n.resolvedLanguage);
                  const missing = (criticalAudit.missing_keys ?? []).includes(key);
                  return (
                    <Badge key={key} variant={missing ? 'warning' : 'success'}>
                      {label}
                    </Badge>
                  );
                })}
              </div>
            </div>
          ) : null}

          {profileAudit.sections.length ? (
            <div className="mb-6 flex flex-wrap gap-2">
              {profileAudit.sections.map((section) => (
                <Badge key={section.key} variant={sectionVariant[section.status]}>
                  {sectionLabels[section.key]}: {t(`feedLibrary.section.${section.status}`)}
                </Badge>
              ))}
            </div>
          ) : null}

          <div className="space-y-4">
            {detailSections.map((section) => (
              <NutrientSection key={section.title} title={section.title}>
                {section.rows.map((row) => (
                  <NutrientRow
                    key={row.id}
                    label={row.label}
                    value={row.value}
                    indent={row.indent}
                  />
                ))}
              </NutrientSection>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-[--border] px-4 py-3">
          <Button variant="ghost" size="sm" onClick={onClose}>
            {t('common.close')}
          </Button>
          <Button variant="outline" size="sm">
            <Icon icon={Edit2} size={14} className="mr-1.5" />
            {t('common.edit')}
          </Button>
          <Button size="sm" onClick={handleAddToRation}>
            <Icon icon={Plus} size={14} className="mr-1.5" />
            {t('feedLibrary.addToRation')}
          </Button>
        </div>
      </div>
    </div>
  );
}

function InfoCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[--radius-md] bg-[--bg-surface] p-3">
      <div className="mb-1 text-[10px] uppercase tracking-wide text-[--text-secondary]">
        {label}
      </div>
      <div className="text-sm font-medium text-[--text-primary]">{value}</div>
    </div>
  );
}

function NutrientSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <h4 className="mb-2 text-xs font-medium uppercase tracking-wide text-[--text-secondary]">
        {title}
      </h4>
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 rounded-[--radius-md] bg-[--bg-surface] p-3">
        {children}
      </div>
    </div>
  );
}

function NutrientRow({
  label,
  value,
  indent,
}: {
  label: string;
  value: string;
  indent?: boolean;
}) {
  return (
    <div className="flex justify-between py-0.5 text-xs">
      <span className={`text-[--text-secondary] ${indent ? 'pl-3' : ''}`}>{label}</span>
      <span className="text-[--text-primary]">{value}</span>
    </div>
  );
}
