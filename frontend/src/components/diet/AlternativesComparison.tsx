import React from 'react';
import { useTranslation } from 'react-i18next';
import { getNutrientLabel, resolveNutrientLanguage, type NutrientLanguage } from '../../lib/nutrient-registry';
import { getNutrientDisplayActual, nutrientDisplayUnit, getManagedNormEntries } from '../../lib/nutrient-display';
import { getNutrientStatus, type NutrientStatus } from '../../lib/nutrient-status';
import type { AlternativeRationSolution } from '../../types/optimization';
import type { NormRange } from '../../types/nutrient';
import type { OptimizedItem } from '../../types/ration';

interface AlternativesComparisonProps {
  solutions: AlternativeRationSolution[];
  currentIndex: number;
  groupId: string;
  norms: Record<string, NormRange>;
  onSelect: (index: number) => void;
}

export function AlternativesComparison({ solutions, currentIndex, groupId, norms, onSelect }: AlternativesComparisonProps) {
  const { t, i18n } = useTranslation();
  const lang = resolveNutrientLanguage(i18n.language);

  if (solutions.length <= 1) return null;

  return (
    <section className="rounded-lg border border-[--border] bg-[--bg-surface] overflow-hidden">
      <h3 className="px-4 py-2 text-sm font-semibold text-[--text-primary] border-b border-[--border]">
        {t('optimizer.solutionComparison', 'Solution Comparison')}
      </h3>

      <SolutionHeadersAndFeeds
        alternatives={solutions}
        currentIndex={currentIndex}
        selectAlternative={onSelect}
      />

      <NutrientComparisonSection
        alternatives={solutions}
        currentIndex={currentIndex}
        lang={lang}
        groupId={groupId}
        norms={norms}
      />
    </section>
  );
}

/* ---------- Solution Headers + Feed Lists ---------- */

interface SectionProps {
  alternatives: AlternativeRationSolution[];
  currentIndex: number;
  selectAlternative: (index: number) => void;
}

function SolutionHeadersAndFeeds({ alternatives, currentIndex, selectAlternative }: SectionProps) {
  const { t } = useTranslation();
  return (
    <div className="overflow-x-auto">
      <div
        className="grid min-w-max"
        style={{
          gridTemplateColumns: `260px repeat(${alternatives.length}, minmax(180px, 1fr))`,
        }}
      >
        {/* Row 1: Header row */}
        <div className="sticky left-0 z-10 bg-[--bg-surface] px-4 py-2 text-xs font-medium text-[--text-secondary] border-b border-[--border] flex items-end">
          {t('optimizer.solutions', 'Solutions')}
        </div>
        {alternatives.map((sol, idx) => (
          <SolutionHeaderCard
            key={sol.id}
            solution={sol}
            index={idx}
            isActive={idx === currentIndex}
            onSelect={() => selectAlternative(idx)}
          />
        ))}

        {/* Row 2: Feed lists */}
        <div className="sticky left-0 z-10 bg-[--bg-surface] px-4 py-2 text-xs font-medium text-[--text-secondary] border-b border-[--border]">
          {t('optimizer.feeds', 'Feeds')}
        </div>
        {alternatives.map((sol, idx) => (
          <FeedList
            key={sol.id}
            feeds={sol.feeds}
            isActive={idx === currentIndex}
          />
        ))}
      </div>
    </div>
  );
}

/* ---------- Solution Header Card ---------- */

interface HeaderCardProps {
  solution: AlternativeRationSolution;
  index: number;
  isActive: boolean;
  onSelect: () => void;
}

function SolutionHeaderCard({ solution, index, isActive, onSelect }: HeaderCardProps) {
  const { t } = useTranslation();
  const scoreColor =
    solution.adequacy_score >= 90
      ? 'text-emerald-400'
      : solution.adequacy_score >= 70
        ? 'text-yellow-400'
        : 'text-red-400';

  return (
    <div
      className={`px-3 py-2 border-b border-[--border] ${
        isActive ? 'bg-[--bg-active] ring-1 ring-inset ring-blue-500/40' : 'bg-[--bg-surface]'
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-sm font-medium text-[--text-primary]">
          {solution.label || `#${index + 1}`}
        </span>
        {!isActive && (
          <button
            onClick={onSelect}
            className="text-[10px] px-2 py-0.5 rounded bg-blue-600 hover:bg-blue-500 text-white transition-colors"
          >
            {t('optimizer.apply', 'Apply')}
          </button>
        )}
        {isActive && (
          <span className="text-[10px] px-2 py-0.5 rounded bg-[--bg-hover] text-[--text-secondary]">
            {t('optimizer.active', 'Active')}
          </span>
        )}
      </div>
      <div className="flex gap-3 text-[11px] text-[--text-secondary]">
        <span>{solution.feeds.length} {t('optimizer.feedsCount', 'feeds')}</span>
        <span>{solution.cost.toFixed(0)} &#8381;</span>
        <span className={scoreColor}>{solution.adequacy_score.toFixed(0)}%</span>
      </div>
      {solution.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-1">
          {solution.tags.map((tag) => (
            <span key={tag} className="text-[9px] px-1.5 py-0.5 rounded bg-[--bg-hover] text-[--text-tertiary]">
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/* ---------- Feed List ---------- */

interface FeedListProps {
  feeds: OptimizedItem[];
  isActive: boolean;
}

function FeedList({ feeds, isActive }: FeedListProps) {
  return (
    <div
      className={`px-3 py-2 border-b border-[--border] space-y-0.5 ${
        isActive ? 'bg-[--bg-active]/50' : ''
      }`}
    >
      {feeds.map((f) => (
        <div key={f.feed_id} className="flex justify-between text-[11px]">
          <span className="text-[--text-primary] truncate mr-2">{f.feed_name}</span>
          <span className="text-[--text-secondary] shrink-0">{f.amount_kg.toFixed(1)} kg</span>
        </div>
      ))}
      {feeds.length === 0 && (
        <span className="text-[11px] text-[--text-tertiary] italic">No feeds</span>
      )}
    </div>
  );
}

/* ---------- Nutrient Comparison Section ---------- */

interface NutrientSectionProps {
  alternatives: AlternativeRationSolution[];
  currentIndex: number;
  lang: NutrientLanguage;
  groupId: string;
  norms: Record<string, NormRange>;
}

function NutrientComparisonSection({ alternatives, currentIndex, lang, groupId, norms }: NutrientSectionProps) {
  const { t } = useTranslation();
  const entries = getManagedNormEntries(groupId, norms)
    .filter(([key]) => key !== 'selenium');

  if (entries.length === 0) return null;

  return (
    <div className="max-h-[50vh] overflow-y-auto overflow-x-auto border-t border-[--border]">
      <div
        className="grid min-w-max"
        style={{
          gridTemplateColumns: `260px repeat(${alternatives.length}, minmax(180px, 1fr))`,
        }}
      >
        {/* Column headers row */}
        <div className="sticky top-0 left-0 z-20 bg-[--bg-surface] px-4 py-1.5 text-xs font-medium text-[--text-secondary] border-b border-[--border]">
          {t('optimizer.nutrients', 'Nutrients')}
        </div>
        {alternatives.map((sol, idx) => (
          <div
            key={sol.id}
            className={`sticky top-0 z-10 px-3 py-1.5 text-[11px] font-medium border-b border-[--border] ${
              idx === currentIndex
                ? 'bg-[--bg-active] text-[--text-primary]'
                : 'bg-[--bg-surface] text-[--text-secondary]'
            }`}
          >
            {sol.label || `#${idx + 1}`}
          </div>
        ))}

        {/* Nutrient rows */}
        {entries.map(([key, norm]) => (
          <NutrientBarRow
            key={key}
            nutrientKey={key}
            norm={norm}
            alternatives={alternatives}
            currentIndex={currentIndex}
            groupId={groupId}
            lang={lang}
          />
        ))}
      </div>
    </div>
  );
}

/* ---------- Nutrient Bar Row ---------- */

interface BarRowProps {
  nutrientKey: string;
  norm: NormRange;
  alternatives: AlternativeRationSolution[];
  currentIndex: number;
  groupId: string;
  lang: NutrientLanguage;
}

function NutrientBarRow({ nutrientKey, norm, alternatives, currentIndex, groupId, lang }: BarRowProps) {
  const label = getNutrientLabel(nutrientKey, lang);
  const unit = nutrientDisplayUnit(groupId, nutrientKey, lang);

  const actuals = alternatives.map((sol) =>
    getNutrientDisplayActual(sol.nutrients, groupId, nutrientKey) ?? 0
  );

  const normMax = norm.max ?? norm.target ?? 0;
  const refMax = Math.max(normMax, ...actuals) || 1;

  return (
    <React.Fragment>
      <div className="sticky left-0 z-10 bg-[--bg-surface] px-4 py-1 border-b border-[--border-subtle] flex flex-col justify-center">
        <span className="text-[11px] text-[--text-primary] leading-tight truncate" title={label}>
          {label}
        </span>
        <span className="text-[9px] text-[--text-tertiary] leading-tight">
          {norm.min != null && `${norm.min.toFixed(1)}`}
          {norm.min != null && norm.max != null && ' \u2013 '}
          {norm.max != null && `${norm.max.toFixed(1)}`}
          {unit && ` ${unit}`}
        </span>
      </div>

      {actuals.map((actual, idx) => {
        const status = getNutrientStatus(actual, norm.min ?? undefined, norm.max ?? undefined, norm.target ?? undefined);
        return (
          <NutrientBarCell
            key={alternatives[idx].id}
            actual={actual}
            refMax={refMax}
            normMin={norm.min}
            normMax={norm.max}
            status={status}
            isActive={idx === currentIndex}
          />
        );
      })}
    </React.Fragment>
  );
}

/* ---------- Nutrient Bar Cell ---------- */

interface BarCellProps {
  actual: number;
  refMax: number;
  normMin: number | undefined;
  normMax: number | undefined;
  status: NutrientStatus;
  isActive: boolean;
}

const STATUS_COLORS: Record<NutrientStatus, string> = {
  ok: 'bg-emerald-500',
  low: 'bg-yellow-500',
  high: 'bg-red-500',
  critical: 'bg-red-600',
};

function NutrientBarCell({ actual, refMax, normMin, normMax, status, isActive }: BarCellProps) {
  const barPct = Math.min((actual / refMax) * 100, 100);
  const normMinPct = normMin != null ? Math.min((normMin / refMax) * 100, 100) : undefined;
  const normMaxPct = normMax != null ? Math.min((normMax / refMax) * 100, 100) : undefined;
  const barColor = STATUS_COLORS[status] ?? STATUS_COLORS.ok;

  return (
    <div
      className={`px-3 py-1 border-b border-[--border-subtle] flex items-center gap-2 ${
        isActive ? 'bg-[--bg-active]/40' : ''
      }`}
    >
      <div className="relative h-4 flex-1 bg-[--bg-hover] rounded-sm overflow-hidden">
        {normMinPct != null && normMaxPct != null && (
          <div
            className="absolute inset-y-0 bg-[--border]/40"
            style={{ left: `${normMinPct}%`, width: `${normMaxPct - normMinPct}%` }}
          />
        )}
        <div
          className={`absolute inset-y-0 left-0 ${barColor} rounded-sm transition-all duration-200`}
          style={{ width: `${barPct}%`, opacity: 0.8 }}
        />
      </div>
      <span className="text-[10px] text-[--text-secondary] shrink-0 w-14 text-right tabular-nums">
        {actual > 0 ? actual.toFixed(1) : '\u2014'}
      </span>
    </div>
  );
}
