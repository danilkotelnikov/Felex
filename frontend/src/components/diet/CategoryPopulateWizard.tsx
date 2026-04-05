import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronDown, ChevronUp, ListChecks, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface WizardFeedOption {
  /** Feed id (local or backend) */
  id: string;
  name: string;
  amount_kg: number;
  /** Optional pre-formatted cost string, e.g. "₽12.50" */
  cost?: string;
}

export interface WizardCategory {
  /** Machine-readable category key, e.g. "roughage" */
  key: string;
  /** Human-readable label shown in the accordion header */
  label: string;
  /** Whether this category must have a selection before Apply is enabled */
  required?: boolean;
  options: WizardFeedOption[];
}

export interface CategoryPopulateWizardProps {
  categories: WizardCategory[];
  /** Pre-selected option ids, keyed by category key */
  initialSelections?: Record<string, string>;
  onApply: (selections: Record<string, string>) => void;
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function formatAmount(amount: number, locale: string): string {
  const isRu = locale.startsWith('ru');
  const formatted = amount % 1 === 0
    ? amount.toFixed(0)
    : amount.toFixed(2);
  return isRu ? `${formatted} кг` : `${formatted} kg`;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface CategoryAccordionProps {
  category: WizardCategory;
  selectedId: string | undefined;
  onSelect: (optionId: string) => void;
  locale: string;
  t: (key: string) => string;
}

function CategoryAccordion({
  category,
  selectedId,
  onSelect,
  locale,
  t,
}: CategoryAccordionProps) {
  const [open, setOpen] = useState(true);
  const hasSelection = selectedId !== undefined;

  return (
    <div className="overflow-hidden rounded-[--radius-md] border border-[--border]">
      {/* Accordion header */}
      <button
        onClick={() => setOpen((prev) => !prev)}
        className={cn(
          'flex w-full items-center justify-between px-3 py-2 text-left transition-colors',
          'bg-[--bg-surface] hover:bg-[--bg-hover]',
        )}
      >
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-[--text-primary]">
            {category.label}
            {category.required ? (
              <span className="ml-0.5 text-[--status-error]">*</span>
            ) : null}
          </span>
          {hasSelection ? (
            <span className="rounded-full bg-[--accent] px-1.5 py-0.5 text-[10px] font-medium text-[--text-inverse]">
              {t('wizard.selected')}
            </span>
          ) : null}
        </div>
        <Icon
          icon={open ? ChevronUp : ChevronDown}
          size={14}
          className="text-[--text-secondary]"
        />
      </button>

      {/* Accordion body */}
      {open ? (
        <div className="divide-y divide-[--border] border-t border-[--border]">
          {category.options.length === 0 ? (
            <p className="px-3 py-2 text-xs text-[--text-secondary]">
              {t('wizard.noOptions')}
            </p>
          ) : (
            category.options.map((option) => {
              const isSelected = option.id === selectedId;
              return (
                <button
                  key={option.id}
                  onClick={() => onSelect(option.id)}
                  className={cn(
                    'flex w-full items-center gap-3 px-3 py-2 text-left text-xs transition-colors',
                    isSelected
                      ? 'bg-[--bg-active]'
                      : 'bg-[--bg-base] hover:bg-[--bg-hover]',
                  )}
                >
                  {/* Radio indicator */}
                  <span
                    className={cn(
                      'flex h-3 w-3 shrink-0 items-center justify-center rounded-full border-2',
                      isSelected ? 'border-[--accent]' : 'border-[--text-disabled]',
                    )}
                  >
                    {isSelected ? (
                      <span className="h-1.5 w-1.5 rounded-full bg-[--accent]" />
                    ) : null}
                  </span>

                  {/* Feed name */}
                  <span className="flex-1 font-medium text-[--text-primary]">
                    {option.name}
                  </span>

                  {/* Amount */}
                  <span className="shrink-0 text-[--text-secondary]">
                    {formatAmount(option.amount_kg, locale)}
                  </span>

                  {/* Cost (optional) */}
                  {option.cost ? (
                    <span className="shrink-0 text-[--text-secondary]">
                      {option.cost}
                    </span>
                  ) : null}
                </button>
              );
            })
          )}
        </div>
      ) : null}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function CategoryPopulateWizard({
  categories,
  initialSelections = {},
  onApply,
  onClose,
}: CategoryPopulateWizardProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;

  const [selections, setSelections] = useState<Record<string, string>>(
    initialSelections,
  );

  const handleSelect = (categoryKey: string, optionId: string) => {
    setSelections((prev) => ({
      ...prev,
      [categoryKey]: optionId,
    }));
  };

  // Apply is disabled when any required category has no selection
  const requiredUnmet = categories.some(
    (cat) => cat.required && selections[cat.key] === undefined,
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Dialog */}
      <div className="relative mx-4 flex max-h-[90vh] w-full max-w-lg flex-col overflow-hidden rounded-[--radius-lg] border border-[--border] bg-[--bg-base] shadow-xl">
        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-[--border] px-4 py-3">
          <div className="flex items-center gap-2">
            <Icon icon={ListChecks} size={16} className="text-[--text-secondary]" />
            <h2 className="text-sm font-medium text-[--text-primary]">
              {t('wizard.buildRation')}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="rounded p-1 text-[--text-secondary] transition-colors hover:bg-[--bg-hover]"
          >
            <Icon icon={X} size={16} />
          </button>
        </div>

        {/* Body – scrollable */}
        <div className="flex-1 space-y-2 overflow-y-auto p-4">
          {categories.length === 0 ? (
            <p className="rounded-[--radius-md] bg-[--bg-surface] px-3 py-4 text-center text-xs text-[--text-secondary]">
              {t('wizard.noCategories')}
            </p>
          ) : (
            categories.map((cat) => (
              <CategoryAccordion
                key={cat.key}
                category={cat}
                selectedId={selections[cat.key]}
                onSelect={(optionId) => handleSelect(cat.key, optionId)}
                locale={locale}
                t={t}
              />
            ))
          )}

          {/* Required legend */}
          {categories.some((c) => c.required) ? (
            <p className="pt-1 text-[10px] text-[--text-secondary]">
              <span className="text-[--status-error]">*</span>{' '}
              {t('wizard.requiredLegend')}
            </p>
          ) : null}
        </div>

        {/* Footer */}
        <div className="flex shrink-0 items-center justify-end gap-2 border-t border-[--border] px-4 py-3">
          <Button variant="ghost" size="sm" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            size="sm"
            disabled={requiredUnmet}
            onClick={() => onApply(selections)}
          >
            {t('wizard.applySelection')}
          </Button>
        </div>
      </div>
    </div>
  );
}
