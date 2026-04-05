import { CheckCircle2, XCircle, TrendingUp, TrendingDown } from 'lucide-react';
import { Icon } from '../ui/Icon';
import type { NutrientStatus } from '@/types/nutrient';

const STATUS_META: Record<NutrientStatus, { icon: typeof CheckCircle2; label: string; color: string }> = {
  ok: { icon: CheckCircle2, label: 'ok', color: 'var(--status-ok)' },
  low: { icon: TrendingDown, label: 'low', color: 'var(--status-warn)' },
  high: { icon: TrendingUp, label: 'high', color: 'var(--status-warn)' },
  critical: { icon: XCircle, label: 'critical', color: 'var(--status-error)' },
};

interface NutrientRowProps {
  name: string;
  actual: number;
  totalActual?: number;
  normMin?: number;
  normOpt?: number;
  normMax?: number;
  unit: string;
  status: NutrientStatus;
  targetPercent?: number;
  showBar?: boolean;
}

function formatValue(value: number | undefined): string {
  if (value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  const abs = Math.abs(value);
  const decimals = abs >= 100 ? 0 : abs >= 10 ? 1 : abs >= 1 ? 2 : 3;
  return value.toFixed(decimals).replace(/\.?0+$/, '');
}

function formatMeasurement(value: number | undefined, unit: string): string {
  const formatted = formatValue(value);
  if (formatted === '-') {
    return formatted;
  }
  return unit ? `${formatted} ${unit}` : formatted;
}

function formatReference(
  normMin: number | undefined,
  normOpt: number | undefined,
  normMax: number | undefined,
  unit: string,
): string {
  const parts: string[] = [];

  if (normOpt !== undefined) {
    parts.push(`= ${formatMeasurement(normOpt, unit)}`);
  }

  if (normMin !== undefined || normMax !== undefined) {
    if (normMin !== undefined && normMax !== undefined) {
      parts.push(`${formatMeasurement(normMin, unit)} - ${formatMeasurement(normMax, unit)}`);
    } else if (normMin !== undefined) {
      parts.push(`>= ${formatMeasurement(normMin, unit)}`);
    } else if (normMax !== undefined) {
      parts.push(`<= ${formatMeasurement(normMax, unit)}`);
    }
  }

  return parts.join(' | ') || '-';
}

export function NutrientRow({
  name,
  actual,
  totalActual,
  normMin,
  normOpt,
  normMax,
  unit,
  status,
  targetPercent,
  showBar,
}: NutrientRowProps) {
  const meta = STATUS_META[status];
  const shouldShowBar = showBar ?? targetPercent !== undefined;
  const percent = targetPercent;
  const barWidth = Math.min(percent ?? 0, 100);
  const actualText = [
    formatMeasurement(actual, unit),
    totalActual !== undefined && totalActual !== actual ? formatMeasurement(totalActual, unit) : null,
  ].filter(Boolean).join(' | ');
  const referenceText = formatReference(normMin, normOpt, normMax, unit);

  return (
    <div className="flex items-center gap-2 border-b border-[--border] py-1.5 text-xs last:border-b-0">
      <Icon icon={meta.icon} size={14} style={{ color: meta.color }} />

      <span className="w-32 truncate text-[--text-primary]">{name}</span>

      {shouldShowBar && (
        <div className="flex-1 h-1.5 overflow-hidden rounded-full bg-[--bg-hover]">
          <div
            className="h-full rounded-full transition-all duration-300"
            style={{
              width: `${Math.min(barWidth, 100)}%`,
              background: meta.color,
            }}
          />
        </div>
      )}

      <span className="w-52 text-right text-[--text-secondary]">
        <span className="block">{actualText}</span>
        <span className="block text-[10px] text-[--text-disabled]">{referenceText}</span>
      </span>

      <span className="w-14 text-right font-medium" style={{ color: meta.color }}>
        {percent !== undefined ? `${percent.toFixed(0)}%` : '-'}
      </span>
    </div>
  );
}
