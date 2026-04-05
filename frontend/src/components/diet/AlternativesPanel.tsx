import { ChevronDown, ChevronUp, ExternalLink, Sparkles } from 'lucide-react';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import { cn } from '@/lib/utils';
import { alternativeDisplayLabel, alternativeTagTone, localizeAlternativeStrategy, localizeAlternativeTag } from '@/lib/alternative-display';
import { formatNutrientValue, getSolutionNutrientProfile } from '@/lib/solution-nutrients';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import { Icon } from '../ui/Icon';
import type { AlternativeRationSolution } from '@/types/optimization';
import type { NormRange } from '@/types/nutrient';

interface AlternativesPanelProps {
  solutions: AlternativeRationSolution[];
  currentIndex: number;
  groupId: string;
  norms: Record<string, NormRange>;
  expanded: boolean;
  onSelect: (index: number) => void;
  onToggleExpanded: () => void;
  onOpenWindow: () => void;
}

export function AlternativesPanel({
  solutions,
  currentIndex,
  groupId,
  norms,
  expanded,
  onSelect,
  onToggleExpanded,
  onOpenWindow,
}: AlternativesPanelProps) {
  const { t, i18n } = useTranslationWithFallback();
  const locale = i18n.resolvedLanguage ?? i18n.language;

  if (solutions.length <= 1) {
    return null;
  }

  const currentSolution = solutions[currentIndex] ?? solutions[0];

  return (
    <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <Icon icon={Sparkles} size={15} className="text-[--accent]" />
            <h3 className="text-sm font-medium text-[--text-primary]">
              {t('optimizer.alternatives')}
            </h3>
            <Badge variant="info">
              {t('optimizer.availableRations', { count: solutions.length })}
            </Badge>
          </div>
          <p className="mt-1 text-xs text-[--text-secondary]">
            {t('optimizer.windowHint')}
          </p>
        </div>

        <Button variant="outline" size="sm" onClick={onOpenWindow}>
          <Icon icon={ExternalLink} size={14} className="mr-1.5" />
          {t('optimizer.openWindow')}
        </Button>
      </div>

      <div className="mt-3 rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="min-w-0">
            <div className="text-[10px] uppercase tracking-wide text-[--text-disabled]">
              {t('optimizer.selectedRation')}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium text-[--text-primary]">
                {alternativeDisplayLabel(currentIndex, t)}
              </span>
              <Badge variant="success">{t('optimizer.currentApplied')}</Badge>
              {currentSolution.tags.slice(0, 2).map((tag) => (
                <Badge key={tag} variant={alternativeTagTone(tag)}>
                  {localizeAlternativeTag(tag, t)}
                </Badge>
              ))}
            </div>
          </div>

          <button
            onClick={onToggleExpanded}
            className="flex items-center gap-1 rounded-[--radius-sm] px-2 py-1 text-xs text-[--text-secondary] transition-colors hover:bg-[--bg-hover] hover:text-[--text-primary]"
          >
            {expanded ? t('common.hideDetails') : t('common.showDetails')}
            <Icon icon={expanded ? ChevronUp : ChevronDown} size={14} />
          </button>
        </div>

        <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-[--text-secondary]">
          <span>
            {t('optimizer.feeds')}: {currentSolution.feeds.length}
          </span>
          <span>
            {t('dashboard.matchScore')}: {formatNutrientValue(currentSolution.adequacy_score, locale)}
          </span>
          <span>
            {t('optimizer.strategy')}: {localizeAlternativeStrategy(currentSolution.applied_strategy, t)}
          </span>
        </div>
      </div>

      {expanded ? (
        <div className="mt-3 space-y-2">
          {solutions.map((solution, index) => (
            <SolutionRow
              key={solution.id}
              solution={solution}
              index={index}
              isSelected={index === currentIndex}
              groupId={groupId}
              norms={norms}
              locale={locale}
              onSelect={() => onSelect(index)}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}

interface SolutionRowProps {
  solution: AlternativeRationSolution;
  index: number;
  isSelected: boolean;
  groupId: string;
  norms: Record<string, NormRange>;
  locale: string;
  onSelect: () => void;
}

function SolutionRow({
  solution,
  index,
  isSelected,
  groupId,
  norms,
  locale,
  onSelect,
}: SolutionRowProps) {
  const { t } = useTranslationWithFallback();
  const nutrients = getSolutionNutrientProfile(solution.nutrients, groupId, norms, t).slice(0, 4);

  return (
    <div
      className={cn(
        'rounded-[--radius-md] border px-3 py-3 transition-colors',
        isSelected
          ? 'border-[--accent] bg-[--bg-active]'
          : 'border-[--border] bg-[--bg-base]',
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium text-[--text-primary]">
              {alternativeDisplayLabel(index, t)}
            </span>
            {isSelected ? (
              <Badge variant="success">{t('optimizer.currentApplied')}</Badge>
            ) : null}
            {solution.tags.map((tag) => (
              <Badge key={tag} variant={alternativeTagTone(tag)}>
                {localizeAlternativeTag(tag, t)}
              </Badge>
            ))}
          </div>

          <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-[--text-secondary]">
            <span>
              {t('optimizer.feeds')}: {solution.feeds.length}
            </span>
            <span>
              {t('dashboard.matchScore')}: {formatNutrientValue(solution.adequacy_score, locale)}
            </span>
            <span>
              {t('optimizer.strategy')}: {localizeAlternativeStrategy(solution.applied_strategy, t)}
            </span>
          </div>

          <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-xs text-[--text-secondary]">
            {nutrients.map((nutrient) => (
              <span key={nutrient.key}>
                {nutrient.abbr} {formatNutrientValue(nutrient.value, locale)} {nutrient.unit}
              </span>
            ))}
          </div>
        </div>

        <Button
          variant={isSelected ? 'ghost' : 'outline'}
          size="sm"
          onClick={onSelect}
        >
          {isSelected ? t('optimizer.currentApplied') : t('optimizer.apply')}
        </Button>
      </div>
    </div>
  );
}
