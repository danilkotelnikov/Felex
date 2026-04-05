import { Button } from '@/components/ui/Button';
import { useTranslationWithFallback } from '@/lib/auto-translate';
import type { MatchedPresetSubcategory } from '@/lib/api';
import type { Feed } from '@/types/feed';
import { capitalizeFirst } from '@/lib/text-utils';

function formatFeedNutrientProfile(
  feed: Feed,
  species: string,
  t: (key: string, options?: Record<string, unknown>) => string,
  locale: string,
): string {
  // Select nutrients based on species
  const nutrientKeys: { key: string; unit: string }[] = (() => {
    switch (species) {
      case 'cattle':
        return [
          { key: 'energy_oe_cattle', unit: 'mj_kg' },
          { key: 'crude_protein', unit: 'g_kg' },
          { key: 'calcium', unit: 'g_kg' },
          { key: 'phosphorus', unit: 'g_kg' },
        ];
      case 'swine':
        return [
          { key: 'energy_oe_pig', unit: 'mj_kg' },
          { key: 'lysine', unit: 'g_kg' },
          { key: 'crude_protein', unit: 'g_kg' },
          { key: 'phosphorus', unit: 'g_kg' },
        ];
      case 'poultry':
        return [
          { key: 'energy_oe_poultry', unit: 'mj_kg' },
          { key: 'methionine_cystine', unit: 'g_kg' },
          { key: 'crude_protein', unit: 'g_kg' },
          { key: 'phosphorus', unit: 'g_kg' },
        ];
      default:
        return [];
    }
  })();

  const parts: string[] = [];
  const decimalSep = locale.startsWith('ru') ? ',' : '.';

  for (const { key, unit } of nutrientKeys) {
    const value = feed[key as keyof Feed];
    if (typeof value === 'number' && Number.isFinite(value)) {
      const abbr = t(`nutrients.abbr.${key}`, { defaultValue: key });
      const unitLabel = t(`nutrients.profileUnits.${unit}`, { defaultValue: unit });
      const formatted = value.toFixed(1).replace('.', decimalSep);
      parts.push(`${abbr}: ${formatted} ${unitLabel}`);
    }
  }

  return parts.length > 0 ? parts.join(', ') : t(`feedCategories.${feed.category}`, { defaultValue: feed.category });
}

type FeedRecommendationsProps = {
  preset: MatchedPresetSubcategory | null;
  species: string;
  onQuickStart: () => void;
  onCustomize: () => void;
  busy?: boolean;
};

export function FeedRecommendations({
  preset,
  species,
  onQuickStart,
  onCustomize,
  busy = false,
}: FeedRecommendationsProps) {
  const { t, i18n } = useTranslationWithFallback();

  if (!preset) {
    return (
      <section className="bg-[--bg-surface] rounded-[--radius-md] p-4">
        <p className="text-xs text-[--text-secondary]">
          {t('dashboard.noPresetSelected')}
        </p>
      </section>
    );
  }

  return (
    <section className="bg-[--bg-surface] rounded-[--radius-md] p-4 space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h3 className="text-xs font-medium text-[--text-secondary] uppercase tracking-wide">
            {t('dashboard.feedRecommendations')}
          </h3>
          <div className="text-sm font-medium text-[--text-primary]">
            {i18n.language.startsWith('en') ? preset.name_en : preset.name_ru}
          </div>
          <div className="text-[10px] text-[--text-secondary]">
            {t('dashboard.presetSource')}: {preset.research_source || t('dashboard.sourceUnavailable')}
          </div>
        </div>

        <div className="flex gap-2">
          <Button variant="outline" size="sm" disabled={busy} onClick={onCustomize}>
            {t('dashboard.customize')}
          </Button>
          <Button size="sm" disabled={busy} onClick={onQuickStart}>
            {t('dashboard.quickStart')}
          </Button>
        </div>
      </div>

      <div className="space-y-3">
        {preset.recommendations.map((recommendation) => (
          <div
            key={recommendation.key}
            className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-3"
          >
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="text-xs font-medium text-[--text-primary]">
                {i18n.language.startsWith('en')
                  ? recommendation.label_en
                  : recommendation.label_ru}
              </div>
              <div className="text-[10px] text-[--text-secondary]">
                {t('dashboard.feedMatches')}: {recommendation.matches.length}
              </div>
            </div>

            {recommendation.matches.length > 0 ? (
              <div className="mt-3 space-y-2">
                {recommendation.matches.slice(0, 3).map((match) => (
                  <div
                    key={`${recommendation.key}-${match.feed.id}`}
                    className="rounded-[--radius-sm] border border-[--border] px-3 py-2"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <span className="text-xs text-[--text-primary]">
                        {capitalizeFirst(match.feed.name_ru)}
                      </span>
                      <span className="text-[10px] text-[--accent]">
                        {t('dashboard.matchScore')}: {Math.round(match.match_score * 100)}%
                      </span>
                    </div>
                    <div className="mt-1 text-[10px] text-[--text-secondary]">
                      {formatFeedNutrientProfile(match.feed, species, t, i18n.language)}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="mt-3 text-[10px] text-[--text-secondary]">
                {t('dashboard.noRecommendationMatches')}
              </p>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}

export default FeedRecommendations;
