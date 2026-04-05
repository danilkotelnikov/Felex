import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Save } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { cn } from '@/lib/utils';
import type { AnimalProperties } from '@/stores/rationStore';

export type { AnimalProperties };

interface AnimalPropertyEditorProps {
  properties: AnimalProperties;
  onSave: (properties: AnimalProperties) => void;
  onClose: () => void;
}

const BREEDS: Record<string, string[]> = {
  cattle_dairy: ['Голштинская', 'Чёрно-пёстрая', 'Симментальская', 'Айрширская', 'Джерсейская', 'Ярославская', 'Красная степная', 'Холмогорская'],
  cattle_beef: ['Абердин-ангусская', 'Герефордская', 'Лимузинская', 'Шароле', 'Казахская белоголовая', 'Калмыцкая'],
  swine_fattening: ['Крупная белая', 'Ландрас', 'Дюрок', 'Пьетрен', 'Гемпшир', 'Гибрид (синт. линия)'],
  swine_breeding: ['Крупная белая', 'Ландрас', 'Дюрок', 'Пьетрен', 'Гемпшир'],
  poultry_broiler: ['Росс 308', 'Кобб 500', 'Арбор Эйкрз', 'Хаббард Флекс', 'Смена-9'],
  poultry_layer: ['Хайсекс Браун', 'Ломанн Браун', 'ISA Brown', 'Тетра SL', 'Пушкинская'],
};

const WEIGHT_HINTS: Record<string, string> = {
  cattle_dairy: '450-750',
  cattle_beef: '250-700',
  swine_fattening: '20-120',
  swine_breeding: '160-280',
  poultry_broiler: '0.1-4.0',
  poultry_layer: '1.4-2.5',
};

export function AnimalPropertyEditor({ properties, onSave, onClose }: AnimalPropertyEditorProps) {
  const { t } = useTranslation();
  const [form, setForm] = useState<AnimalProperties>(properties);

  const profileKey = `${form.species}_${form.productionType}`;
  const breeds = BREEDS[profileKey] ?? [];
  const weightHint = WEIGHT_HINTS[profileKey] ?? '0-1000';
  const currentAgeDays = form.ageToDays ?? form.ageFromDays;

  const handleChange = <K extends keyof AnimalProperties>(key: K, value: AnimalProperties[K]) => {
    setForm((previous) => ({ ...previous, [key]: value }));
  };

  const parseOptionalNumber = (value: string) => {
    if (value.trim() === '') {
      return undefined;
    }

    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : undefined;
  };

  const handleAgeChange = (value: string) => {
    const age = Number.parseInt(value, 10);
    setForm((previous) => ({
      ...previous,
      ageFromDays: Number.isFinite(age) ? age : undefined,
      ageToDays: Number.isFinite(age) ? age : undefined,
    }));
  };

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    onSave(form);
    onClose();
  };

  const showDairyFields = form.species === 'cattle' && form.productionType === 'dairy';
  const showGrowthFields = form.productionType === 'beef' || form.productionType === 'fattening' || form.productionType === 'broiler';
  const showAgeField = form.species === 'poultry' || form.productionType === 'fattening' || form.productionType === 'beef';
  const showLayerFields = form.productionType === 'layer';
  const showSwineBreedingFields = form.species === 'swine' && form.productionType === 'breeding';

  const getSexLabel = (sex: 'male' | 'female' | 'mixed') => {
    const key = `animal.${form.species}_${sex}`;
    const translated = t(key);
    return translated === key ? t(`animal.${sex}`) : translated;
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-[--bg-surface] rounded-[--radius-lg] shadow-xl w-full max-w-md" onClick={(event) => event.stopPropagation()}>
        <div className="flex items-center justify-between px-4 py-3 border-b border-[--border]">
          <h2 className="text-sm font-medium text-[--text-primary]">{t('animal.editProperties')}</h2>
          <button onClick={onClose} className="p-1 rounded hover:bg-[--bg-hover] text-[--text-secondary]">
            <Icon icon={X} size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          <div>
            <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">{t('animal.breed')}</label>
            <select
              value={form.breed}
              onChange={(event) => handleChange('breed', event.target.value)}
              className="w-full px-3 py-2 text-xs bg-[--bg-base] border border-[--border] rounded-[--radius-md] text-[--text-primary]"
            >
              {breeds.map((breed) => (
                <option key={breed} value={breed}>{breed}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
              {t('animal.liveWeight')} ({t('units.kg')})
            </label>
            <Input
              type="number"
              value={form.liveWeightKg}
              onChange={(event) => handleChange('liveWeightKg', Number.parseFloat(event.target.value) || 0)}
              className="w-full text-xs"
              min={0}
              step={form.species === 'poultry' ? 0.1 : 1}
            />
            <p className="text-[10px] text-[--text-disabled] mt-1">
              {t('animal.normRange')}: {weightHint} {t('units.kg')}
            </p>
          </div>

          <div>
            <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">{t('animal.sex')}</label>
            <div className="flex gap-2">
              {(['female', 'male', 'mixed'] as const).map((sex) => (
                <button
                  key={sex}
                  type="button"
                  onClick={() => handleChange('sex', sex)}
                  className={cn(
                    'flex-1 px-3 py-2 text-xs rounded-[--radius-md] border transition-colors',
                    form.sex === sex
                      ? 'border-[--accent] bg-[--bg-active] text-[--accent]'
                      : 'border-[--border] bg-[--bg-base] text-[--text-secondary] hover:border-[--text-disabled]'
                  )}
                >
                  {getSexLabel(sex)}
                </button>
              ))}
            </div>
          </div>

          {showAgeField ? (
            <div>
              <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
                {t('norms.age')} ({t('common.day')})
              </label>
              <Input
                type="number"
                value={currentAgeDays ?? ''}
                onChange={(event) => handleAgeChange(event.target.value)}
                className="w-full text-xs"
                min={0}
                step={1}
              />
            </div>
          ) : null}

          {showDairyFields ? (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
                  {t('animal.milkYield')} ({t('units.kg')}/{t('common.day')})
                </label>
                <Input
                  type="number"
                  value={form.milkYieldKg ?? ''}
                  onChange={(event) => handleChange('milkYieldKg', parseOptionalNumber(event.target.value))}
                  className="w-full text-xs"
                  min={0}
                  max={80}
                  step={1}
                />
              </div>
              <div>
                <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">{t('animal.milkFat')} (%)</label>
                <Input
                  type="number"
                  value={form.milkFatPct ?? ''}
                  onChange={(event) => handleChange('milkFatPct', parseOptionalNumber(event.target.value))}
                  className="w-full text-xs"
                  min={2}
                  max={6}
                  step={0.1}
                />
              </div>
            </div>
          ) : null}

          {showGrowthFields ? (
            <div>
              <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
                {t('animal.dailyGain')} ({t('units.g')}/{t('common.day')})
              </label>
              <Input
                type="number"
                value={form.dailyGainG ?? ''}
                onChange={(event) => handleChange('dailyGainG', parseOptionalNumber(event.target.value))}
                className="w-full text-xs"
                min={0}
                max={form.species === 'poultry' ? 120 : 2000}
                step={form.species === 'poultry' ? 5 : 50}
              />
            </div>
          ) : null}

          {showLayerFields ? (
            <div>
              <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
                {t('animal.eggProduction')} ({t('animal.eggsPerYear')})
              </label>
              <Input
                type="number"
                value={form.eggProductionPerYear ?? ''}
                onChange={(event) => handleChange('eggProductionPerYear', parseOptionalNumber(event.target.value))}
                className="w-full text-xs"
                placeholder="280-340"
                min={0}
                max={400}
                step={10}
              />
            </div>
          ) : null}

          {showSwineBreedingFields ? (
            <>
              <div>
                <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">{t('animal.stage')}</label>
                <div className="flex gap-2">
                  {([
                    { id: 'gestation', label: t('animal.gestation') },
                    { id: 'lactation', label: t('animal.lactation') },
                  ] as const).map((stage) => (
                    <button
                      key={stage.id}
                      type="button"
                      onClick={() => handleChange('reproductiveStage', stage.id)}
                      className={cn(
                        'flex-1 px-3 py-2 text-xs rounded-[--radius-md] border transition-colors',
                        form.reproductiveStage === stage.id
                          ? 'border-[--accent] bg-[--bg-active] text-[--accent]'
                          : 'border-[--border] bg-[--bg-base] text-[--text-secondary] hover:border-[--text-disabled]'
                      )}
                    >
                      {stage.label}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <label className="text-xs font-medium text-[--text-primary] mb-1.5 block">
                  {t('animal.litterSize')} ({t('animal.piglets')})
                </label>
                <Input
                  type="number"
                  value={form.litterSize ?? ''}
                  onChange={(event) => handleChange('litterSize', parseOptionalNumber(event.target.value))}
                  className="w-full text-xs"
                  min={0}
                  max={20}
                  step={1}
                />
              </div>
            </>
          ) : null}

          <div className="flex gap-2 pt-2">
            <Button type="button" variant="outline" size="sm" className="flex-1" onClick={onClose}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" size="sm" className="flex-1">
              <Icon icon={Save} size={14} className="mr-1.5" />
              {t('common.save')}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}

export function AnimalPropertySummary({
  properties,
  onEdit,
}: {
  properties: AnimalProperties;
  onEdit: () => void;
}) {
  const { t } = useTranslation();

  const getSummaryParts = () => {
    const parts: string[] = [properties.breed];

    if (properties.species === 'cattle' && properties.productionType === 'dairy' && properties.milkYieldKg) {
      parts.push(`${properties.milkYieldKg} ${t('units.kg')}/${t('common.day')}`);
    }

    if (properties.species === 'poultry' && properties.productionType === 'layer' && properties.eggProductionPerYear) {
      parts.push(`${properties.eggProductionPerYear} ${t('animal.eggsPerYear')}`);
    }

    if (properties.dailyGainG && (properties.productionType === 'beef' || properties.productionType === 'fattening' || properties.productionType === 'broiler')) {
      parts.push(`${properties.dailyGainG} ${t('units.g')}/${t('common.day')}`);
    }

    if (properties.species === 'swine' && properties.productionType === 'breeding') {
      parts.push(properties.reproductiveStage === 'gestation' ? t('animal.gestation') : t('animal.lactation'));
      if (properties.litterSize) {
        parts.push(`${properties.litterSize} ${t('animal.piglets')}`);
      }
    }

    parts.push(`${properties.liveWeightKg} ${t('units.kg')}`);

    return parts.filter(Boolean).join(' | ');
  };

  return (
    <button onClick={onEdit} className="text-left hover:text-[--accent] transition-colors" title={t('animal.editProperties')}>
      <p className="text-xs text-[--text-secondary]">{getSummaryParts()}</p>
    </button>
  );
}
