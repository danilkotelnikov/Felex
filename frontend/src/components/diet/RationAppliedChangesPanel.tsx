import { useTranslation } from 'react-i18next';
import { Badge } from '../ui/Badge';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getFeedDisplayNameFromCatalog, resolveFeedLanguage } from '@/lib/feed-display';
import { localizeOptimizationReason } from '@/lib/optimization-feedback';
import { formatNumber } from '@/lib/utils';
import { useRationStore } from '@/stores/rationStore';

function formatKg(value: number) {
  if (Math.abs(value) >= 10) {
    return formatNumber(value, 1);
  }

  return formatNumber(value, 2);
}

export function RationAppliedChangesPanel() {
  const { t, i18n } = useTranslation();
  const optimizationFeedback = useRationStore((state) => state.optimizationFeedback);
  const { feeds: feedCatalog } = useFeedCatalog();
  const language = resolveFeedLanguage(i18n.resolvedLanguage);

  if (!optimizationFeedback || optimizationFeedback.isStale) {
    return null;
  }

  const autoAddedFeeds = optimizationFeedback.solution.auto_added_feeds ?? [];
  if (!autoAddedFeeds.length) {
    return null;
  }

  const massUnit = i18n.language.startsWith('ru') ? 'кг' : 'kg';

  return (
    <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-xs font-medium text-[--text-primary]">
          {t('ration.appliedChangesTitle')}
        </h3>
        <Badge variant="info">{t('ration.autoAddedCount', { count: autoAddedFeeds.length })}</Badge>
      </div>
      <p className="mt-1 text-xs text-[--text-secondary]">
        {t('ration.appliedChangesDesc')}
      </p>

      <div className="mt-3 grid gap-2 md:grid-cols-2">
        {autoAddedFeeds.map((feed) => (
          <div key={feed.feed_id} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
            <div className="text-xs font-medium text-[--text-primary]">
              {getFeedDisplayNameFromCatalog(feed.feed_id, feed.feed_name, feedCatalog, language)} ({formatKg(feed.amount_kg)} {massUnit})
            </div>
            {feed.reasons.length ? (
              <div className="mt-1 text-[11px] leading-4 text-[--text-secondary]">
                {feed.reasons
                  .slice(0, 2)
                  .map((reason) => localizeOptimizationReason(reason, t))
                  .join(' ')}
              </div>
            ) : null}
          </div>
        ))}
      </div>
    </section>
  );
}
