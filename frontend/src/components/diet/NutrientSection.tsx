import { Children, useState, type ReactNode } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useTranslationWithFallback } from '@/lib/auto-translate';

interface NutrientSectionProps {
  id: string;
  titleKey: string;
  defaultOpen?: boolean;
  children: ReactNode;
}

export function NutrientSection({
  id,
  titleKey,
  defaultOpen = false,
  children,
}: NutrientSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const { t } = useTranslationWithFallback();
  const hasContent = Children.count(children) > 0;

  return (
    <section className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface]">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
        onClick={() => setIsOpen((value) => !value)}
        aria-expanded={isOpen}
        aria-controls={`nutrient-section-${id}`}
      >
        <span className="text-sm font-medium text-[--text-primary]">
          {t(titleKey)}
        </span>
        {isOpen ? (
          <ChevronDown className="h-4 w-4 text-[--text-secondary]" />
        ) : (
          <ChevronRight className="h-4 w-4 text-[--text-secondary]" />
        )}
      </button>
      {isOpen && hasContent ? (
        <div id={`nutrient-section-${id}`} className="px-4 pb-3">
          {children}
        </div>
      ) : null}
    </section>
  );
}

export default NutrientSection;
