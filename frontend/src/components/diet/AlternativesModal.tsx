import { useMemo } from 'react';
import { CheckCircle2, Sparkles, X } from 'lucide-react';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { cn } from '@/lib/utils';
import {
  getManagedNormEntries,
  getNutrientDisplayActual,
  getNutrientTargetPercent,
  nutrientDisplayUnit,
} from '@/lib/nutrient-display';
import { getNutrientStatus } from '@/lib/nutrient-status';
import { getNutrientLabel, resolveNutrientLanguage } from '@/lib/nutrient-registry';
import {
  alternativeDisplayLabel,
  alternativeTagTone,
  localizeAlternativeStrategy,
  localizeAlternativeTag,
} from '@/lib/alternative-display';
import { formatNutrientValue } from '@/lib/solution-nutrients';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import { Icon } from '../ui/Icon';
import type { AlternativeRationSolution } from '@/types/optimization';
import type { NormRange } from '@/types/nutrient';

interface AlternativesModalProps {
  solutions: AlternativeRationSolution[];
  currentIndex: number;
  groupId: string;
  norms: Record<string, NormRange>;
  pendingApply?: boolean;
  onSelect: (index: number) => void;
  onClose: () => void;
}

interface ComparisonNutrientRow {
  key: string;
  label: string;
  unit: string;
  norm: NormRange;
  values: Array<number | undefined>;
}

function formatValue(value: number | undefined, locale: string): string {
  if (value === undefined || !Number.isFinite(value)) {
    return '-';
  }

  return formatNutrientValue(value, locale);
}

function nutrientTone(status: ReturnType<typeof getNutrientStatus>): string {
  switch (status) {
    case 'critical':
      return 'var(--status-error)';
    case 'low':
    case 'high':
      return 'var(--status-warn)';
    default:
      return 'var(--status-ok)';
  }
}

export function AlternativesModal({
  solutions,
  currentIndex,
  groupId,
  norms,
  pendingApply = false,
  onSelect,
  onClose,
}: AlternativesModalProps) {
  const { t, i18n } = useTranslationWithFallback();
  const locale = i18n.resolvedLanguage ?? i18n.language;
  const nutrientLanguage = resolveNutrientLanguage(locale);

  const feedRows = useMemo(() => {
    const seen = new Map<string, string>();
    solutions.forEach((solution) => {
      solution.feeds.forEach((feed) => {
        seen.set(`${feed.feed_id}:${feed.feed_name}`, feed.feed_name);
      });
    });

    return Array.from(seen.entries())
      .map(([compositeKey, feedName]) => ({
        compositeKey,
        feedName,
        values: solutions.map((solution) => {
          const item = solution.feeds.find((feed) => `${feed.feed_id}:${feed.feed_name}` === compositeKey);
          return item?.amount_kg;
        }),
      }))
      .sort((left, right) => left.feedName.localeCompare(right.feedName, locale));
  }, [locale, solutions]);

  const nutrientRows = useMemo<ComparisonNutrientRow[]>(() => {
    const entries = [...getManagedNormEntries(groupId, norms)]
      .filter(([key]) => key !== 'selenium');

    return entries.map(([key, norm]) => ({
      key,
      label: getNutrientLabel(key, nutrientLanguage),
      unit: nutrientDisplayUnit(groupId, key, nutrientLanguage),
      norm,
      values: solutions.map((solution) => getNutrientDisplayActual(solution.nutrients, groupId, key)),
    }));
  }, [groupId, norms, nutrientLanguage, solutions]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/45 backdrop-blur-[1px]" onClick={onClose} />

      <div className="relative mx-4 flex max-h-[94vh] w-full max-w-[min(96vw,1480px)] flex-col overflow-hidden rounded-[--radius-lg] border border-[--border] bg-[--bg-base] shadow-xl">
        <div className="flex shrink-0 flex-wrap items-start justify-between gap-3 border-b border-[--border] px-5 py-4">
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <Icon icon={Sparkles} size={16} className="text-[--accent]" />
              <h2 className="text-base font-medium text-[--text-primary]">
                {t('optimizer.compareSolutions')}
              </h2>
              <Badge variant="info">
                {t('optimizer.availableRations', { count: solutions.length })}
              </Badge>
            </div>
            <p className="mt-1 text-xs text-[--text-secondary]">
              {t('optimizer.comparisonSummary')}
            </p>
          </div>

          <button
            onClick={onClose}
            className="rounded-[--radius-sm] p-1.5 text-[--text-secondary] transition-colors hover:bg-[--bg-hover] hover:text-[--text-primary]"
            aria-label={t('common.close')}
          >
            <Icon icon={X} size={16} />
          </button>
        </div>

        <div className="flex-1 overflow-auto p-5">
          <div className="min-w-[1080px] space-y-4">
            <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div>
                  <h3 className="text-sm font-medium text-[--text-primary]">
                    {t('optimizer.alternatives')}
                  </h3>
                  <p className="mt-1 text-xs text-[--text-secondary]">
                    {t('optimizer.clickToApply')}
                  </p>
                </div>
              </div>

              <div
                className="grid gap-3"
                style={{ gridTemplateColumns: `repeat(${solutions.length}, minmax(220px, 1fr))` }}
              >
                {solutions.map((solution, index) => {
                  const isSelected = index === currentIndex;

                  return (
                    <div
                      key={solution.id}
                      className={cn(
                        'rounded-[--radius-md] border p-3',
                        isSelected
                          ? 'border-[--accent] bg-[--bg-active]'
                          : 'border-[--border] bg-[--bg-base]',
                      )}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div>
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="text-sm font-medium text-[--text-primary]">
                              {alternativeDisplayLabel(index, t)}
                            </span>
                            {isSelected && !pendingApply ? (
                              <Badge variant="success">{t('optimizer.currentApplied')}</Badge>
                            ) : null}
                          </div>
                          <div className="mt-2 flex flex-wrap gap-1.5">
                            <Badge variant="secondary">
                              {localizeAlternativeStrategy(solution.applied_strategy, t)}
                            </Badge>
                            {solution.tags.map((tag) => (
                              <Badge key={tag} variant={alternativeTagTone(tag)}>
                                {localizeAlternativeTag(tag, t)}
                              </Badge>
                            ))}
                          </div>
                        </div>

                        {isSelected && !pendingApply ? (
                          <Icon icon={CheckCircle2} size={16} className="text-[--status-ok]" />
                        ) : null}
                      </div>

                      <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
                        <div className="rounded-[--radius-sm] bg-[--bg-surface] px-2 py-2">
                          <div className="text-[10px] uppercase tracking-wide text-[--text-disabled]">
                            {t('optimizer.feeds')}
                          </div>
                          <div className="mt-1 font-medium text-[--text-primary]">
                            {solution.feeds.length}
                          </div>
                        </div>
                        <div className="rounded-[--radius-sm] bg-[--bg-surface] px-2 py-2">
                          <div className="text-[10px] uppercase tracking-wide text-[--text-disabled]">
                            {t('dashboard.matchScore')}
                          </div>
                          <div className="mt-1 font-medium text-[--text-primary]">
                            {formatValue(solution.adequacy_score, locale)}%
                          </div>
                        </div>
                      </div>

                      <div className="mt-2 text-xs text-[--text-secondary]">
                        {t('optimize.resultCostLabel')}: {formatValue(solution.cost, locale)}
                      </div>

                      <Button
                        variant={isSelected && !pendingApply ? 'ghost' : 'default'}
                        size="sm"
                        onClick={() => onSelect(index)}
                        className="mt-3 w-full"
                      >
                        {isSelected && !pendingApply
                          ? t('optimizer.currentApplied')
                          : t('optimizer.applySolution')}
                      </Button>
                    </div>
                  );
                })}
              </div>
            </section>

            <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface]">
              <div className="border-b border-[--border] px-4 py-3">
                <h3 className="text-sm font-medium text-[--text-primary]">
                  {t('optimizer.feeds')}
                </h3>
              </div>

              <div className="overflow-auto">
                <div
                  className="grid text-xs"
                  style={{ gridTemplateColumns: `280px repeat(${solutions.length}, minmax(180px, 1fr))` }}
                >
                  <div className="sticky left-0 z-10 border-r border-[--border] bg-[--bg-surface] px-4 py-3 text-[10px] uppercase tracking-wide text-[--text-disabled]">
                    {t('optimizer.feedColumn')}
                  </div>
                  {solutions.map((solution, index) => (
                    <div
                      key={`feed-head-${solution.id}`}
                      className={cn(
                        'border-l border-[--border] px-4 py-3 text-[10px] uppercase tracking-wide',
                        index === currentIndex
                          ? 'bg-[--bg-active] text-[--accent]'
                          : 'bg-[--bg-surface] text-[--text-disabled]',
                      )}
                    >
                      {alternativeDisplayLabel(index, t)}
                    </div>
                  ))}

                  {feedRows.map((row) => (
                    <div key={row.compositeKey} className="contents">
                      <div className="sticky left-0 z-10 border-r border-t border-[--border] bg-[--bg-surface] px-4 py-3 text-[--text-primary]">
                        {row.feedName}
                      </div>
                      {row.values.map((value, index) => (
                        <div
                          key={`${row.compositeKey}-${solutions[index].id}`}
                          className={cn(
                            'border-l border-t border-[--border] px-4 py-3 text-[--text-secondary]',
                            index === currentIndex ? 'bg-[--bg-active]/40' : '',
                          )}
                        >
                          {value !== undefined ? `${formatValue(value, locale)} ${t('units.kg')}` : '-'}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            </section>

            <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface]">
              <div className="flex flex-wrap items-start justify-between gap-3 border-b border-[--border] px-4 py-3">
                <div>
                  <h3 className="text-sm font-medium text-[--text-primary]">
                    {t('workspace.nutrients')}
                  </h3>
                  <p className="mt-1 text-xs text-[--text-secondary]">
                    {t('optimizer.percentOfTargetHint')}
                  </p>
                </div>
              </div>

              <div className="overflow-auto">
                <div
                  className="grid text-xs"
                  style={{ gridTemplateColumns: `280px repeat(${solutions.length}, minmax(200px, 1fr))` }}
                >
                  <div className="sticky left-0 z-10 border-r border-[--border] bg-[--bg-surface] px-4 py-3 text-[10px] uppercase tracking-wide text-[--text-disabled]">
                    {t('workspace.nutrients')}
                  </div>
                  {solutions.map((solution, index) => (
                    <div
                      key={`nutrient-head-${solution.id}`}
                      className={cn(
                        'border-l border-[--border] px-4 py-3 text-[10px] uppercase tracking-wide',
                        index === currentIndex
                          ? 'bg-[--bg-active] text-[--accent]'
                          : 'bg-[--bg-surface] text-[--text-disabled]',
                      )}
                    >
                      {alternativeDisplayLabel(index, t)}
                    </div>
                  ))}

                  {nutrientRows.map((row) => (
                    <div key={row.key} className="contents">
                      <div className="sticky left-0 z-10 border-r border-t border-[--border] bg-[--bg-surface] px-4 py-3">
                        <div className="text-[--text-primary]">{row.label}</div>
                        <div className="mt-1 text-[10px] text-[--text-disabled]">
                          {row.unit || '\u00A0'}
                        </div>
                      </div>
                      {row.values.map((value, index) => {
                        const status = getNutrientStatus(value ?? 0, row.norm.min, row.norm.max, row.norm.target);
                        const tone = nutrientTone(status);
                        const percent = getNutrientTargetPercent(value ?? 0, row.norm.target);

                        return (
                          <div
                            key={`${row.key}-${solutions[index].id}`}
                            className={cn(
                              'border-l border-t border-[--border] px-4 py-3',
                              index === currentIndex ? 'bg-[--bg-active]/40' : '',
                            )}
                          >
                            <div className="flex items-center justify-between gap-2">
                              <span className="text-[--text-primary]">
                                {value !== undefined ? `${formatValue(value, locale)} ${row.unit}`.trim() : '-'}
                              </span>
                              <span className="h-2 w-2 shrink-0 rounded-full" style={{ backgroundColor: tone }} />
                            </div>

                            {percent !== undefined ? (
                              <div className="mt-2">
                                <div className="h-1.5 overflow-hidden rounded-full bg-[--bg-hover]">
                                  <div
                                    className="h-full rounded-full"
                                    style={{
                                      width: `${Math.min(percent, 100)}%`,
                                      backgroundColor: tone,
                                    }}
                                  />
                                </div>
                                <div className="mt-1 text-[10px] text-[--text-secondary]">
                                  {formatValue(percent, locale)}%
                                </div>
                              </div>
                            ) : null}
                          </div>
                        );
                      })}
                    </div>
                  ))}
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}
