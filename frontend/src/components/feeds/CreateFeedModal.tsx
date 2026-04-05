import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { getFeedCategoryLabel } from '@/lib/feed-categories';
import { getNutrientLabel, resolveNutrientLanguage } from '@/lib/nutrient-registry';
import type { Feed } from '@/types/feed';

interface CreateFeedModalProps {
  onSave: (feed: Feed) => void;
  onClose: () => void;
}

interface NutrientFieldDef {
  key: string;
  unit: string;
}

interface NutrientSection {
  titleKey: string;
  defaultOpen: boolean;
  fields: NutrientFieldDef[];
}

const SECTIONS: NutrientSection[] = [
  {
    titleKey: 'createFeed.basicNutrients',
    defaultOpen: true,
    fields: [
      { key: 'dry_matter', unit: '%' },
      { key: 'koe', unit: 'КЕ' },
      { key: 'energy_oe_cattle', unit: 'MJ/kg' },
      { key: 'energy_oe_pig', unit: 'MJ/kg' },
      { key: 'energy_oe_poultry', unit: 'MJ/kg' },
      { key: 'crude_protein', unit: 'g/kg' },
      { key: 'crude_fiber', unit: 'g/kg' },
      { key: 'crude_fat', unit: 'g/kg' },
    ],
  },
  {
    titleKey: 'createFeed.digestibility',
    defaultOpen: false,
    fields: [
      { key: 'dig_protein_cattle', unit: 'g/kg' },
      { key: 'dig_protein_pig', unit: 'g/kg' },
      { key: 'dig_protein_poultry', unit: 'g/kg' },
      { key: 'sugar', unit: 'g/kg' },
      { key: 'starch', unit: 'g/kg' },
    ],
  },
  {
    titleKey: 'nutrients.proteinAminoAcids',
    defaultOpen: false,
    fields: [
      { key: 'lysine', unit: 'g/kg' },
      { key: 'methionine_cystine', unit: 'g/kg' },
    ],
  },
  {
    titleKey: 'nutrients.minerals',
    defaultOpen: false,
    fields: [
      { key: 'calcium', unit: 'g/kg' },
      { key: 'phosphorus', unit: 'g/kg' },
      { key: 'magnesium', unit: 'g/kg' },
      { key: 'potassium', unit: 'g/kg' },
      { key: 'sodium', unit: 'g/kg' },
      { key: 'iron', unit: 'mg/kg' },
      { key: 'copper', unit: 'mg/kg' },
      { key: 'zinc', unit: 'mg/kg' },
      { key: 'manganese', unit: 'mg/kg' },
      { key: 'cobalt', unit: 'mg/kg' },
      { key: 'iodine', unit: 'mg/kg' },
    ],
  },
  {
    titleKey: 'nutrients.vitamins',
    defaultOpen: false,
    fields: [
      { key: 'carotene', unit: 'mg/kg' },
      { key: 'vit_d3', unit: 'IU/kg' },
      { key: 'vit_e', unit: 'mg/kg' },
    ],
  },
];

export function CreateFeedModal({ onSave, onClose }: CreateFeedModalProps) {
  const { t, i18n } = useTranslation();
  const nutrientLanguage = resolveNutrientLanguage(i18n.resolvedLanguage);

  const [name, setName] = useState('');
  const [category, setCategory] = useState('concentrate');
  const [pricePerTon, setPricePerTon] = useState('');
  const [values, setValues] = useState<Record<string, string>>({ dry_matter: '87' });

  const categoryLanguage = i18n.resolvedLanguage?.startsWith('en') ? 'en' : 'ru';
  const categoryOptions = [
    'grain', 'concentrate', 'oilseed_meal', 'protein', 'roughage', 'silage',
    'succulent', 'green_forage', 'animal_origin', 'mineral', 'premix',
    'oil_fat', 'byproduct', 'compound_feed', 'additive', 'other',
  ].map((value) => ({
    value,
    label: getFeedCategoryLabel(value, categoryLanguage),
  }));

  const setField = (key: string, val: string) => {
    setValues((prev) => ({ ...prev, [key]: val }));
  };

  const parseField = (key: string): number | undefined => {
    const raw = values[key];
    if (!raw) return undefined;
    const num = parseFloat(raw);
    return Number.isFinite(num) && num > 0 ? num : undefined;
  };

  const handleSave = () => {
    if (!name.trim()) return;

    const feed: Feed = {
      id: Date.now(),
      name_ru: name.trim(),
      category,
      is_custom: true,
      dry_matter: parseField('dry_matter') ?? 87,
      koe: parseField('koe'),
      energy_oe_cattle: parseField('energy_oe_cattle'),
      energy_oe_pig: parseField('energy_oe_pig'),
      energy_oe_poultry: parseField('energy_oe_poultry'),
      crude_protein: parseField('crude_protein'),
      crude_fiber: parseField('crude_fiber'),
      crude_fat: parseField('crude_fat'),
      dig_protein_cattle: parseField('dig_protein_cattle'),
      dig_protein_pig: parseField('dig_protein_pig'),
      dig_protein_poultry: parseField('dig_protein_poultry'),
      sugar: parseField('sugar'),
      starch: parseField('starch'),
      lysine: parseField('lysine'),
      methionine_cystine: parseField('methionine_cystine'),
      calcium: parseField('calcium'),
      phosphorus: parseField('phosphorus'),
      magnesium: parseField('magnesium'),
      potassium: parseField('potassium'),
      sodium: parseField('sodium'),
      iron: parseField('iron'),
      copper: parseField('copper'),
      zinc: parseField('zinc'),
      manganese: parseField('manganese'),
      cobalt: parseField('cobalt'),
      iodine: parseField('iodine'),
      carotene: parseField('carotene'),
      vit_d3: parseField('vit_d3'),
      vit_e: parseField('vit_e'),
      price_per_ton: parseFloat(pricePerTon) || undefined,
    };

    onSave(feed);
    onClose();
  };

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-[--bg-surface] rounded-[--radius-lg] shadow-xl w-[520px] max-h-[85vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-[--border]">
          <h2 className="text-sm font-medium text-[--text-primary]">
            {t('feedLibrary.createFeed')}
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-[--bg-hover] text-[--text-disabled]"
          >
            <Icon icon={X} size={16} />
          </button>
        </div>

        {/* Form */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* Name */}
          <div>
            <label className="text-xs font-medium text-[--text-primary] mb-1 block">
              {t('createFeed.name')} *
            </label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('createFeed.namePlaceholder')}
              className="text-xs"
              autoFocus
            />
          </div>

          {/* Category */}
          <div>
            <label className="text-xs font-medium text-[--text-primary] mb-1 block">
              {t('createFeed.category')}
            </label>
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className="w-full px-3 py-2 text-xs bg-[--bg-base] border border-[--border] rounded-[--radius-md] text-[--text-primary]"
            >
              {categoryOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>

          {/* Nutrient sections */}
          {SECTIONS.map((section) => (
            <details key={section.titleKey} open={section.defaultOpen}>
              <summary className="cursor-pointer text-xs font-medium text-[--text-secondary] uppercase tracking-wide hover:text-[--text-primary] list-none flex items-center justify-between py-1">
                {t(section.titleKey)}
                <span className="text-[10px] text-[--text-disabled]">
                  {section.fields.filter((f) => values[f.key]).length}/{section.fields.length}
                </span>
              </summary>
              <div className="grid grid-cols-2 gap-3 mt-2">
                {section.fields.map((field) => (
                  <NumField
                    key={field.key}
                    label={getNutrientLabel(field.key, nutrientLanguage)}
                    unit={field.unit}
                    value={values[field.key] ?? ''}
                    onChange={(v) => setField(field.key, v)}
                  />
                ))}
              </div>
            </details>
          ))}

          {/* Price */}
          <div>
            <h3 className="text-xs font-medium text-[--text-secondary] mb-2 uppercase tracking-wide">
              {t('createFeed.price')}
            </h3>
            <NumField label={t('prices.pricePerTon')} unit="₽" value={pricePerTon} onChange={setPricePerTon} />
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-[--border]">
          <Button variant="ghost" size="sm" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!name.trim()}>
            {t('common.save')}
          </Button>
        </div>
      </div>
    </div>
  );
}

function NumField({
  label, unit, value, onChange,
}: {
  label: string; unit: string; value: string; onChange: (v: string) => void;
}) {
  return (
    <div>
      <label className="text-[10px] text-[--text-secondary] mb-0.5 block">{label}</label>
      <div className="flex items-center gap-1">
        <Input
          type="number"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="text-xs flex-1"
          step="0.1"
          min="0"
        />
        <span className="text-[10px] text-[--text-disabled] w-12">{unit}</span>
      </div>
    </div>
  );
}
