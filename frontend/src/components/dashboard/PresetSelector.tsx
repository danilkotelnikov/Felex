import { useMemo } from 'react';

import { useTranslationWithFallback } from '@/lib/auto-translate';
import type { MatchedPresetCategory, MatchedPresetSubcategory } from '@/lib/api';
import { capitalizeFirst } from '@/lib/text-utils';

type PresetSelectorProps = {
  categories: MatchedPresetCategory[];
  selectedSpecies: string;
  selectedProductionType: string;
  selectedPresetId: string | null;
  onSpeciesChange: (species: string) => void;
  onProductionTypeChange: (productionType: string) => void;
  onPresetChange: (presetId: string) => void;
  loading?: boolean;
};

function speciesLabel(species: string, t: (key: string, fallback?: string) => string) {
  if (species === 'cattle') return t('nav.cattle');
  if (species === 'swine') return t('nav.swine');
  if (species === 'poultry') return t('nav.poultry');
  return species;
}

function productionLabel(
  productionType: string,
  t: (key: string, fallback?: string) => string,
) {
  const keyMap: Record<string, string> = {
    dairy: 'newRation.dairy',
    beef: 'newRation.beef',
    growing: 'dashboard.productionGrowing',
    breeding: 'dashboard.productionBreeding',
    broiler: 'newRation.broiler',
    layer: 'newRation.layer',
  };

  return t(keyMap[productionType] ?? productionType, productionType);
}

function presetLabel(
  preset: MatchedPresetSubcategory,
  language: string,
) {
  const name = language.startsWith('en') ? preset.name_en : preset.name_ru;
  return capitalizeFirst(name) ?? name;
}

export function PresetSelector({
  categories,
  selectedSpecies,
  selectedProductionType,
  selectedPresetId,
  onSpeciesChange,
  onProductionTypeChange,
  onPresetChange,
  loading = false,
}: PresetSelectorProps) {
  const { t, i18n } = useTranslationWithFallback();

  const speciesOptions = useMemo(
    () => Array.from(new Set(categories.map((category) => category.species))),
    [categories],
  );
  const productionOptions = useMemo(
    () =>
      categories
        .filter((category) => category.species === selectedSpecies)
        .map((category) => category.production_type),
    [categories, selectedSpecies],
  );
  const presets = useMemo(
    () =>
      categories.find(
        (category) =>
          category.species === selectedSpecies &&
          category.production_type === selectedProductionType,
      )?.subcategories ?? [],
    [categories, selectedProductionType, selectedSpecies],
  );
  const selectedPreset =
    presets.find((preset) => preset.id === selectedPresetId) ?? presets[0] ?? null;

  if (loading) {
    return (
      <div className="bg-[--bg-surface] rounded-[--radius-md] p-4">
        <p className="text-xs text-[--text-secondary]">
          {t('dashboard.loadingPresets')}
        </p>
      </div>
    );
  }

  if (categories.length === 0) {
    return (
      <div className="bg-[--bg-surface] rounded-[--radius-md] p-4">
        <p className="text-xs text-[--text-secondary]">
          {t('dashboard.noPresets')}
        </p>
      </div>
    );
  }

  return (
    <section className="bg-[--bg-surface] rounded-[--radius-md] p-4 space-y-4">
      <div>
        <h3 className="text-xs font-medium text-[--text-secondary] uppercase tracking-wide">
          {t('dashboard.researchPresets')}
        </h3>
      </div>

      <div className="grid gap-3 md:grid-cols-3">
        <label className="space-y-1">
          <span className="block text-[10px] text-[--text-secondary]">
            {t('dashboard.speciesLabel')}
          </span>
          <select
            value={selectedSpecies}
            onChange={(event) => onSpeciesChange(event.target.value)}
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs text-[--text-primary]"
          >
            {speciesOptions.map((species) => (
              <option key={species} value={species}>
                {speciesLabel(species, t)}
              </option>
            ))}
          </select>
        </label>

        <label className="space-y-1">
          <span className="block text-[10px] text-[--text-secondary]">
            {t('dashboard.productionLabel')}
          </span>
          <select
            value={selectedProductionType}
            onChange={(event) => onProductionTypeChange(event.target.value)}
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs text-[--text-primary]"
          >
            {productionOptions.map((productionType) => (
              <option key={productionType} value={productionType}>
                {productionLabel(productionType, t)}
              </option>
            ))}
          </select>
        </label>

        <label className="space-y-1">
          <span className="block text-[10px] text-[--text-secondary]">
            {t('dashboard.levelLabel')}
          </span>
          <select
            value={selectedPreset?.id ?? ''}
            onChange={(event) => onPresetChange(event.target.value)}
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-2 text-xs text-[--text-primary]"
          >
            {presets.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {presetLabel(preset, i18n.language)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {selectedPreset ? (
        <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] px-3 py-3">
          <div className="text-sm font-medium text-[--text-primary]">
            {presetLabel(selectedPreset, i18n.language)}
          </div>
          <div className="mt-2 flex flex-wrap gap-3 text-[10px] text-[--text-secondary]">
            <span>
              {t('dashboard.matchedFeeds')}: {selectedPreset.matched_feed_count}
            </span>
            <span>
              {t('dashboard.presetSource')}:{' '}
              {selectedPreset.research_source || t('dashboard.sourceUnavailable')}
            </span>
          </div>
        </div>
      ) : null}
    </section>
  );
}

export default PresetSelector;
