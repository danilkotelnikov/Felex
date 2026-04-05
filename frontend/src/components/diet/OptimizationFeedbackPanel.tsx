import { AlertCircle, CheckCircle2, SlidersHorizontal } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AutoAddedFeedsSection } from './AutoAddedFeedsSection';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import { Icon } from '../ui/Icon';
import { useFeedCatalog } from '@/lib/feed-catalog';
import { getFeedDisplayNameFromCatalog, resolveFeedLanguage } from '@/lib/feed-display';
import {
  getRelaxedTargetDisplay,
  localizeOptimizationReason,
  localizeWorkflowNote,
  relaxedTargetSummaryKey,
} from '@/lib/optimization-feedback';
import { cn, formatNumber } from '@/lib/utils';
import { useRationStore } from '@/stores/rationStore';

interface OptimizationFeedbackPanelProps {
  onOpenOptimize?: () => void;
}

function formatMetricValue(value: number) {
  if (Math.abs(value) >= 100) {
    return formatNumber(value, 0);
  }
  if (Math.abs(value) >= 10) {
    return formatNumber(value, 1);
  }
  return formatNumber(value, 2);
}

export function OptimizationFeedbackPanel({ onOpenOptimize }: OptimizationFeedbackPanelProps) {
  const { t, i18n } = useTranslation();
  const optimizationFeedback = useRationStore((state) => state.optimizationFeedback);
  const { feeds: feedCatalog } = useFeedCatalog();

  if (!optimizationFeedback) {
    return null;
  }

  const language = resolveFeedLanguage(i18n.resolvedLanguage);
  const { solution, isStale } = optimizationFeedback;
  const relaxedTargets = solution.relaxed_targets ?? [];
  const recommendations = isStale ? [] : (solution.recommendations ?? []).slice(0, 3);
  const autoAddedFeeds = isStale ? [] : (solution.auto_added_feeds ?? []);
  const tone = solution.best_achievable || isStale ? 'warning' : 'success';
  const message = isStale
    ? t('workspace.optimizationFeedbackStaleDesc')
    : solution.best_achievable
      ? t('workspace.optimizationClosestDesc')
      : t('workspace.optimizationExactDesc');
  const workflowNotes = (solution.workflow_notes ?? []).slice(0, 3).map((note) => localizeWorkflowNote(note, t));
  const massUnit = i18n.language.startsWith('ru') ? 'кг' : 'kg';

  return (
    <section
      className={cn(
        'mb-4 rounded-[--radius-md] border p-4',
        tone === 'warning'
          ? 'border-[--status-warn] bg-[--status-warn-bg]'
          : 'border-[--status-ok] bg-[--status-ok-bg]'
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1 space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Icon
              icon={tone === 'warning' ? AlertCircle : CheckCircle2}
              size={16}
              className={tone === 'warning' ? 'text-[--status-warn]' : 'text-[--status-ok]'}
            />
            <h3 className="text-sm font-medium text-[--text-primary]">
              {t('workspace.optimizationFeedbackTitle')}
            </h3>
            <Badge variant={solution.best_achievable ? 'warning' : 'success'}>
              {solution.best_achievable
                ? t('workspace.optimizationClosest')
                : t('workspace.optimizationExact')}
            </Badge>
            {isStale ? (
              <Badge variant="secondary">{t('workspace.optimizationFeedbackStale')}</Badge>
            ) : null}
          </div>

          <p className="text-xs text-[--text-secondary]">{message}</p>

          {workflowNotes.length ? (
            <ul className="space-y-1 text-xs text-[--text-secondary]">
              {workflowNotes.map((note) => (
                <li key={note} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-1.5">
                  {note}
                </li>
              ))}
            </ul>
          ) : null}
        </div>

        {onOpenOptimize ? (
          <Button variant="outline" size="sm" onClick={onOpenOptimize}>
            <Icon icon={SlidersHorizontal} size={14} className="mr-1.5" />
            {isStale ? t('workspace.reoptimize') : t('workspace.reviewOptimization')}
          </Button>
        ) : null}
      </div>

      {relaxedTargets.length ? (
        <div className="mt-3 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
          <div className="mb-2 text-xs font-medium text-[--text-secondary]">
            {t('optimize.relaxedTargets')}
          </div>
          <div className="grid gap-2 md:grid-cols-2">
            {relaxedTargets.slice(0, 4).map((target) => {
              const meta = getRelaxedTargetDisplay(target, i18n.language);
              return (
                <div key={`${target.key}-${target.constraint_type}`} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2 text-xs">
                  <div className="font-medium text-[--text-primary]">{meta.label}</div>
                  <div className="text-[--text-secondary]">
                    {t(relaxedTargetSummaryKey(target.constraint_type))}: {formatMetricValue(target.actual)}
                    {meta.unit ? ` ${meta.unit}` : ''} / {formatMetricValue(target.target)}
                    {meta.unit ? ` ${meta.unit}` : ''}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      {autoAddedFeeds.length ? (
        <div className="mt-3">
          <AutoAddedFeedsSection
            feeds={autoAddedFeeds}
            titleKey="workspace.autoAddedFeeds"
          />
        </div>
      ) : null}

      {recommendations.length ? (
        <div className="mt-3 rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
          <div className="mb-2 text-xs font-medium text-[--text-secondary]">
            {t('workspace.remainingSuggestions')}
          </div>
          <ul className="space-y-2 text-xs text-[--text-secondary]">
            {recommendations.map((recommendation) => (
              <li key={recommendation.feed_id} className="rounded-[--radius-sm] bg-[--bg-base] px-2 py-2">
                <div className="font-medium text-[--text-primary]">
                  {getFeedDisplayNameFromCatalog(recommendation.feed_id, recommendation.feed_name, feedCatalog, language)} ({formatMetricValue(recommendation.suggested_amount_kg)} {massUnit})
                </div>
                <div>{localizeOptimizationReason(recommendation.reason, t)}</div>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}
