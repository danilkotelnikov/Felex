// frontend/src/components/diet/SolutionCard.tsx
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronDown, ChevronUp, Check, AlertTriangle, Minus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import type { AlternativeRationSolution, NutrientStatusInfo } from '@/types/optimization';
import {
  formatNutrientValue,
  getSolutionNutrientProfile,
} from '@/lib/solution-nutrients';
import type { NormRange } from '@/types/nutrient';

interface SolutionCardProps {
  solution: AlternativeRationSolution;
  index: number;
  isSelected: boolean;
  groupId: string;
  norms: Record<string, NormRange>;
  onApply: () => void;
  compact?: boolean;
}

const MAX_VISIBLE_FEEDS = 4;

const STATUS_ICONS = {
  ok: Check,
  near: Minus,
  out: AlertTriangle,
};

const STATUS_COLORS = {
  ok: 'text-[--status-ok]',
  near: 'text-[--status-warn]',
  out: 'text-[--status-error]',
};

export function SolutionCard({
  solution,
  index,
  isSelected,
  groupId,
  norms,
  onApply,
  compact = false,
}: SolutionCardProps) {
  const { t, i18n } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const locale = i18n.language;

  const nutrients = getSolutionNutrientProfile(
    solution.nutrients,
    groupId,
    norms,
    t
  );

  const visibleFeeds = solution.feeds.slice(0, MAX_VISIBLE_FEEDS);
  const hiddenCount = solution.feeds.length - MAX_VISIBLE_FEEDS;
  const totalWeight = solution.feeds.reduce((sum, f) => sum + f.amount_kg, 0);

  const label = index === 0
    ? t('optimizer.current')
    : t('optimizer.solution', { n: index + 1 });

  return (
    <div
      className={cn(
        'flex flex-col rounded-[--radius-md] border p-3',
        isSelected
          ? 'border-[--accent] bg-[--bg-active]'
          : 'border-[--border] bg-[--bg-surface]'
      )}
    >
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-[--text-primary]">
            {label}
          </span>
          {isSelected && (
            <span className="h-2 w-2 rounded-full bg-[--accent]" />
          )}
        </div>
      </div>

      {!compact && (
        <>
          <div className="mb-2 text-xs text-[--text-secondary]">
            {t('optimizer.feeds')}:
          </div>
          <div className="mb-2 space-y-1">
            {visibleFeeds.map((feed) => {
              const pct = totalWeight > 0
                ? ((feed.amount_kg / totalWeight) * 100).toFixed(0)
                : '0';
              return (
                <div
                  key={feed.feed_id}
                  className="flex justify-between text-xs text-[--text-secondary]"
                >
                  <span className="truncate">{feed.feed_name}</span>
                  <span className="ml-2 shrink-0">
                    {formatNutrientValue(feed.amount_kg, locale)} {t('units.kg')} ({pct}%)
                  </span>
                </div>
              );
            })}
            {hiddenCount > 0 && (
              <div className="text-xs text-[--text-disabled]">
                {t('optimizer.moreFeeds', { count: hiddenCount })}
              </div>
            )}
          </div>
        </>
      )}

      <button
        onClick={() => setExpanded(!expanded)}
        className="mb-2 flex items-center gap-1 text-xs text-[--text-secondary] hover:text-[--text-primary]"
      >
        <Icon icon={expanded ? ChevronUp : ChevronDown} size={12} />
        {t('optimizer.nutrientComparison')}
      </button>

      {expanded && (
        <div className="mb-3 space-y-1 rounded-[--radius-sm] bg-[--bg-base] p-2">
          {nutrients.map((n) => (
            <NutrientRow key={n.key} nutrient={n} locale={locale} />
          ))}
        </div>
      )}

      <Button
        size="sm"
        variant={isSelected ? 'ghost' : 'default'}
        onClick={onApply}
        className="w-full"
      >
        {t('optimizer.apply')}
      </Button>
    </div>
  );
}

function NutrientRow({
  nutrient,
  locale,
}: {
  nutrient: NutrientStatusInfo;
  locale: string;
}) {
  const StatusIcon = STATUS_ICONS[nutrient.status];

  return (
    <div className="flex items-center justify-between text-xs">
      <span className="text-[--text-secondary]">
        {nutrient.abbr}: {formatNutrientValue(nutrient.value, locale)} {nutrient.unit}
      </span>
      <Icon
        icon={StatusIcon}
        size={12}
        className={STATUS_COLORS[nutrient.status]}
      />
    </div>
  );
}
