import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { MarkdownContent } from '../ui/MarkdownContent';
import {
  formatMethodologyFactorValue,
  formatMethodologyMetricPair,
  getNormMethodologyCitationMarkdown,
  getNormMethodologyCopy,
  getNormMethodologyFactorLabel,
  getNormMethodologyMetricLabel,
} from '@/lib/norm-methodology';
import type { NormMethodology } from '@/types/nutrient';

interface NormMethodologyCardProps {
  methodology?: NormMethodology | null;
  normSource: 'backend' | 'local';
  showFormulaMarkdown?: boolean;
  showCitationMarkdown?: boolean;
  emptyStateMessage?: string | null;
}

export function NormMethodologyCard({
  methodology,
  normSource,
  showFormulaMarkdown = true,
  showCitationMarkdown = true,
  emptyStateMessage = null,
}: NormMethodologyCardProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage || i18n.language;
  const copy = useMemo(() => getNormMethodologyCopy(methodology, t), [methodology, t]);
  const citationMarkdown = useMemo(
    () => getNormMethodologyCitationMarkdown(methodology, language),
    [language, methodology],
  );

  if (!methodology && !emptyStateMessage && normSource !== 'local') {
    return null;
  }

  const description = methodology
    ? copy.description
    : emptyStateMessage ?? t('norms.methodologyUnavailable');

  return (
    <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-xs font-medium text-[--text-primary]">{t('norms.methodology')}</div>
          {methodology ? (
            <div className="mt-1 text-xs text-[--text-primary]">{copy.title}</div>
          ) : null}
          <div className="mt-1 text-[10px] leading-5 text-[--text-secondary]">{description}</div>
        </div>
        {methodology ? (
          <div className="rounded-full border border-[--border] px-2 py-1 text-[10px] text-[--text-secondary]">
            {methodology.dynamic ? t('norms.methodologyDynamicBadge') : t('norms.methodologyStaticBadge')}
          </div>
        ) : null}
      </div>

      <details className="group mt-4">
        <summary className="cursor-pointer text-xs font-medium text-[--text-primary] hover:text-[--accent] list-none flex items-center justify-between">
          {t('common.showDetails')}
          <span className="text-[10px] text-[--text-disabled] group-open:hidden">▼</span>
          <span className="text-[10px] text-[--text-disabled] hidden group-open:inline">▲</span>
        </summary>
        <div className="mt-4 space-y-4">
          {methodology?.source_refs?.length ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologySources')}
              </div>
              <div className="mt-2 space-y-1 text-xs text-[--text-secondary]">
                {methodology.source_refs.map((item) => (
                  <div key={item}>{item}</div>
                ))}
              </div>
            </div>
          ) : null}

          {methodology?.driver_metrics?.length ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologyDrivers')}
              </div>
              <div className="mt-2 grid gap-2 md:grid-cols-2">
                {methodology.driver_metrics.map((metric) => {
                  const pair = formatMethodologyMetricPair(metric, language, t);
                  return (
                    <div key={metric.key} className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] p-3">
                      <div className="text-xs font-medium text-[--text-primary]">
                        {getNormMethodologyMetricLabel(metric.key, t)}
                      </div>
                      <div className="mt-2 flex items-start justify-between gap-3 text-[10px]">
                        <div>
                          <div className="text-[--text-secondary]">{t('norms.methodologyReferenceValue')}</div>
                          <div className="text-[--text-primary]">
                            {pair.reference} {pair.unit}
                          </div>
                        </div>
                        <div className="text-right">
                          <div className="text-[--text-secondary]">{t('norms.methodologyCurrentValue')}</div>
                          <div className="text-[--text-primary]">
                            {pair.current} {pair.unit}
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}

          {methodology?.derived_metrics?.length ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologyDerived')}
              </div>
              <div className="mt-2 grid gap-2 md:grid-cols-2">
                {methodology.derived_metrics.map((metric) => {
                  const pair = formatMethodologyMetricPair(metric, language, t);
                  return (
                    <div key={metric.key} className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] p-3">
                      <div className="text-xs font-medium text-[--text-primary]">
                        {getNormMethodologyMetricLabel(metric.key, t)}
                      </div>
                      <div className="mt-2 flex items-start justify-between gap-3 text-[10px]">
                        <div>
                          <div className="text-[--text-secondary]">{t('norms.methodologyReferenceValue')}</div>
                          <div className="text-[--text-primary]">
                            {pair.reference} {pair.unit}
                          </div>
                        </div>
                        <div className="text-right">
                          <div className="text-[--text-secondary]">{t('norms.methodologyCurrentValue')}</div>
                          <div className="text-[--text-primary]">
                            {pair.current} {pair.unit}
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}

          {methodology?.scaling_factors?.length ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologyFactors')}
              </div>
              <div className="mt-2 grid gap-2 md:grid-cols-2">
                {methodology.scaling_factors.map((factor) => (
                  <div key={factor.key} className="flex items-center justify-between rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs">
                    <span className="text-[--text-primary]">{getNormMethodologyFactorLabel(factor.key, t)}</span>
                    <span className="text-[--text-secondary]">{formatMethodologyFactorValue(factor, language)}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {methodology && showFormulaMarkdown ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologyCalc')}
              </div>
              <MarkdownContent markdown={copy.formulaMarkdown} variant="chat" className="mt-2" />
            </div>
          ) : null}

          {methodology && showCitationMarkdown && citationMarkdown ? (
            <div>
              <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('norms.methodologyBibliography')}
              </div>
              <MarkdownContent markdown={citationMarkdown} variant="chat" className="mt-2" />
            </div>
          ) : null}
        </div>
      </details>
    </div>
  );
}
