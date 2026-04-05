import { useTranslation } from 'react-i18next';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getFeedDisplayNameFromCatalog, resolveFeedLanguage } from '@/lib/feed-display';
import { localizeOptimizationReason } from '@/lib/optimization-feedback';
import type { AutoAddedFeed } from '@/types/ration';
import { formatNumber } from '@/lib/utils';

interface AutoAddedFeedsSectionProps {
  feeds: AutoAddedFeed[];
  titleKey: string;
}

function formatKg(value: number) {
  if (Math.abs(value) >= 10) {
    return formatNumber(value, 1);
  }
  return formatNumber(value, 2);
}

export function AutoAddedFeedsSection({ feeds, titleKey }: AutoAddedFeedsSectionProps) {
  const { t, i18n } = useTranslation();
  const massUnit = i18n.language.startsWith('ru') ? 'кг' : 'kg';
  const language = resolveFeedLanguage(i18n.resolvedLanguage);
  const { feeds: feedCatalog } = useFeedCatalog();

  if (!feeds.length) {
    return null;
  }

  return (
    <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
      <h4 className="mb-2 text-xs font-medium text-[--text-secondary]">
        {t(titleKey)}
      </h4>
      <ul className="space-y-2 text-xs text-[--text-secondary]">
        {feeds.map((feed) => (
          <li key={feed.feed_id} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
            <div className="font-medium text-[--text-primary]">
              {getFeedDisplayNameFromCatalog(feed.feed_id, feed.feed_name, feedCatalog, language)} ({formatKg(feed.amount_kg)} {massUnit})
            </div>
            {feed.reasons.slice(0, 3).map((reason) => (
              <div key={reason}>{localizeOptimizationReason(reason, t)}</div>
            ))}
          </li>
        ))}
      </ul>
    </div>
  );
}
