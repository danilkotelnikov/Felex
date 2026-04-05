import type { ReactNode } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { ExternalLink, X } from 'lucide-react';

import { Button } from '../ui/Button';
import { Icon } from '../ui/Icon';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { getFeedCategoryLabel } from '@/lib/feed-categories';
import type { PriceAnchorSource, PriceProvenance } from '@/lib/api';

export interface PriceDialogRow {
  feedId: number;
  feedName: string;
  category: string;
  subcategory: string | null;
  pricePerTon: number | null;
  provenance: PriceProvenance | null;
  lastUpdated: string | null;
  region: string | null;
}

interface PriceProvenanceDialogProps {
  row: PriceDialogRow | null;
  open: boolean;
  onClose: () => void;
  onOpenSource: (url: string) => void;
}

function benchmarkFamilyForCategory(category: string): string {
  switch (category) {
    case 'roughage':
    case 'silage':
    case 'succulent':
    case 'green_forage':
      return 'forage';
    case 'grain':
    case 'concentrate':
    case 'compound_feed':
    case 'byproduct':
      return 'energy';
    case 'oilseed_meal':
    case 'protein':
    case 'animal_origin':
      return 'protein';
    case 'mineral':
    case 'additive':
      return 'mineral';
    case 'premix':
      return 'premix';
    case 'oil_fat':
      return 'fat';
    default:
      return 'other';
  }
}

function localizeAnchorSource(anchor: PriceAnchorSource, t: (key: string, fallback?: string) => string): string {
  if (anchor.kind === 'domain') {
    return anchor.label;
  }

  if (anchor.kind === 'manual') {
    return t('prices.anchorKinds.manual', 'Manual entries');
  }

  if (anchor.kind === 'seed') {
    return t('prices.anchorKinds.seed', 'Base catalog');
  }

  if (anchor.kind === 'other') {
    return anchor.label;
  }

  return t('prices.anchorKinds.unknown', 'Unknown source');
}

function getBenchmarkBasisLabel(
  row: PriceDialogRow,
  provenance: PriceProvenance,
  language: 'ru' | 'en',
  t: (key: string, fallback?: string) => string,
): string | null {
  switch (provenance.benchmark_level) {
    case 'subcategory':
      return row.subcategory ?? t('prices.noDash', '-');
    case 'category':
      return getFeedCategoryLabel(row.category, language) || row.category || null;
    case 'family':
      return t(`prices.benchmarkFamilies.${benchmarkFamilyForCategory(row.category)}`, 'Feed family');
    case 'global':
      return t('prices.benchmarkFamilies.global', 'All available anchors');
    default:
      return null;
  }
}

function provenanceBadgeClass(kind: string | null | undefined): string {
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

export function PriceProvenanceDialog({
  row,
  open,
  onClose,
  onOpenSource,
}: PriceProvenanceDialogProps) {
  const { t, i18n } = useTranslationWithFallback();
  const language = i18n.resolvedLanguage?.startsWith('en') ? 'en' : 'ru';
  const provenance = row?.provenance ?? null;
  const provenanceKind = provenance?.kind ?? 'unknown';
  const provenanceLabel = t(`prices.provenanceKinds.${provenanceKind}`, provenanceKind);
  const benchmarkBasis = row && provenance
    ? getBenchmarkBasisLabel(row, provenance, language, t)
    : null;

  return (
    <Dialog.Root open={open} onOpenChange={(nextOpen) => { if (!nextOpen) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/40" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-[min(640px,92vw)] max-h-[85vh] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-[--border] bg-[--bg-surface] shadow-xl">
          <div className="flex items-start justify-between gap-4 border-b border-[--border] px-5 py-4">
            <div className="min-w-0">
              <Dialog.Title className="text-sm font-medium text-[--text-primary]">
                {t('prices.detailsTitle', 'Price provenance')}
              </Dialog.Title>
              <div className="mt-1 text-xs text-[--text-secondary]">
                {row?.feedName ?? t('prices.noDash', '-')}
              </div>
            </div>
            <Dialog.Close asChild>
              <button
                type="button"
                className="rounded p-1 text-[--text-disabled] hover:bg-[--bg-hover] hover:text-[--text-primary]"
                aria-label={t('common.close', 'Close')}
              >
                <Icon icon={X} size={16} />
              </button>
            </Dialog.Close>
          </div>

          <div className="space-y-4 px-5 py-4">
            {!row || !provenance ? (
              <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3 text-sm text-[--text-secondary]">
                {t('prices.detailsUnavailable', 'No provenance details available.')}
              </div>
            ) : (
              <>
                <div className="grid gap-3 sm:grid-cols-2">
                  <DetailField
                    label={t('prices.provenanceType', 'Type')}
                    value={(
                      <span className={`inline-flex rounded px-2 py-0.5 text-[11px] ${provenanceBadgeClass(provenanceKind)}`}>
                        {provenanceLabel}
                      </span>
                    )}
                  />
                  <DetailField
                    label={t('prices.lastUpdated', 'Last updated')}
                    value={row.lastUpdated ?? t('prices.noDash', '-')}
                  />
                  <DetailField
                    label={t('prices.category', 'Category')}
                    value={getFeedCategoryLabel(row.category, language) || row.category || t('prices.noDash', '-')}
                  />
                  <DetailField
                    label={t('prices.region', 'Region')}
                    value={row.region ?? t('prices.noDash', '-')}
                  />
                </div>

                {provenance.kind === 'direct' ? (
                  <section className="space-y-3 rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3">
                    <p className="text-sm text-[--text-secondary]">
                      {t('prices.directDetails', 'This price was taken from a direct parsed source.')}
                    </p>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <DetailField
                        label={t('prices.sourceSite', 'Source site')}
                        value={provenance.source_domain ?? t('prices.noDash', '-')}
                      />
                      <DetailField
                        label={t('prices.provenancePrecision', 'Precision')}
                        value={provenance.is_precise_source
                          ? t('prices.preciseSource', 'Exact source page')
                          : t('prices.noPreciseSource', 'No precise source page available.')}
                      />
                    </div>
                    {provenance.source_url ? (
                      <div className="flex justify-start">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => onOpenSource(provenance.source_url!)}
                        >
                          <Icon icon={ExternalLink} size={14} className="mr-1.5" />
                          {t('prices.openSourcePage', 'Open source page')}
                        </Button>
                      </div>
                    ) : null}
                  </section>
                ) : null}

                {provenance.kind === 'benchmark' ? (
                  <section className="space-y-3 rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3">
                    <p className="text-sm text-[--text-secondary]">
                      {t('prices.benchmarkDetails', 'This price was derived from benchmark anchors.')}
                    </p>
                    <div className="grid gap-3 sm:grid-cols-2">
                      <DetailField
                        label={t('prices.benchmarkLevel', 'Benchmark level')}
                        value={t(
                          `prices.benchmarkLevels.${provenance.benchmark_level ?? 'unknown'}`,
                          provenance.benchmark_level ?? 'Unknown',
                        )}
                      />
                      <DetailField
                        label={t('prices.anchorCount', 'Anchor prices')}
                        value={String(provenance.anchor_count ?? 0)}
                      />
                      <DetailField
                        label={t('prices.benchmarkBasis', 'Benchmark basis')}
                        value={benchmarkBasis ?? t('prices.noDash', '-')}
                      />
                    </div>
                    <div>
                      <div className="mb-2 text-[11px] font-medium uppercase tracking-wide text-[--text-secondary]">
                        {t('prices.anchorSources', 'Anchor sources')}
                      </div>
                      {provenance.anchor_sources?.length ? (
                        <div className="flex flex-wrap gap-2">
                          {provenance.anchor_sources.map((anchor) => (
                            <span
                              key={`${anchor.kind}:${anchor.label}`}
                              className="inline-flex items-center gap-1 rounded-full border border-[--border] bg-[--bg-surface] px-2 py-1 text-[11px] text-[--text-primary]"
                            >
                              <span>{localizeAnchorSource(anchor, t)}</span>
                              <span className="text-[--text-secondary]">x{anchor.count}</span>
                            </span>
                          ))}
                        </div>
                      ) : (
                        <div className="text-sm text-[--text-secondary]">
                          {t('prices.noAnchorSources', 'No source breakdown available.')}
                        </div>
                      )}
                    </div>
                  </section>
                ) : null}

                {provenance.kind === 'manual' ? (
                  <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3 text-sm text-[--text-secondary]">
                    {t('prices.manualDetails', 'This price was entered manually.')}
                  </section>
                ) : null}

                {provenance.kind === 'seed' ? (
                  <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3 text-sm text-[--text-secondary]">
                    {t('prices.seedDetails', 'This price comes from the base catalog data.')}
                  </section>
                ) : null}

                {provenance.kind === 'unknown' ? (
                  <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-4 py-3 text-sm text-[--text-secondary]">
                    {t('prices.unknownDetails', 'This price source could not be classified.')}
                  </section>
                ) : null}
              </>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function DetailField({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="space-y-1 rounded-[--radius-sm] border border-[--border] bg-[--bg-surface] px-3 py-2">
      <div className="text-[11px] uppercase tracking-wide text-[--text-secondary]">{label}</div>
      <div className="text-sm text-[--text-primary] break-all">{value}</div>
    </div>
  );
}
