import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { DollarSign, Percent, Scale, TrendingUp } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { getFeedDisplayName, resolveFeedLanguage } from '@/lib/feed-display';
import { useRationStore } from '@/stores/rationStore';
import { formatCurrency, formatNumber } from '@/lib/utils';

interface CostBreakdownItem {
  name: string;
  amount_kg: number;
  price_per_kg: number;
  daily_cost: number;
  percentage: number;
}

export function EconomicsPanel() {
  const { t, i18n } = useTranslation();
  const { localItems, nutrients, animalProperties, animalCount } = useRationStore();
  const language = resolveFeedLanguage(i18n.resolvedLanguage);

  const economics = useMemo(() => {
    if (localItems.length === 0) return null;

    const breakdown: CostBreakdownItem[] = localItems.map((item) => {
      const price_per_kg = (item.feed.price_per_ton ?? 0) / 1000;
      const daily_cost = item.amount_kg * price_per_kg;
      return {
        name: getFeedDisplayName(item.feed, language),
        amount_kg: item.amount_kg,
        price_per_kg,
        daily_cost,
        percentage: 0,
      };
    });

    const totalCostPerHead = breakdown.reduce((sum, item) => sum + item.daily_cost, 0);

    breakdown.forEach((item) => {
      item.percentage = totalCostPerHead > 0 ? (item.daily_cost / totalCostPerHead) * 100 : 0;
    });

    breakdown.sort((a, b) => b.daily_cost - a.daily_cost);

    const totalWeight = localItems.reduce((sum, item) => sum + item.amount_kg, 0);
    const totalCost = totalCostPerHead * animalCount;
    const costPerKg = totalWeight > 0 ? totalCostPerHead / totalWeight : 0;
    const costPerEKE = nutrients && nutrients.energy_eke > 0
      ? totalCostPerHead / nutrients.energy_eke
      : 0;
    const cpKg = nutrients ? nutrients.crude_protein / 1000 : 0;
    const costPerCPKg = cpKg > 0 ? totalCostPerHead / cpKg : 0;
    const monthlyCost = totalCost * 30;

    const milkYield = animalProperties.milkYieldKg ?? 0;
    const costPerLiterMilk = milkYield > 0 ? totalCostPerHead / milkYield : 0;

    const dailyGainKg = (animalProperties.dailyGainG ?? 0) / 1000;
    const costPerKgGain = dailyGainKg > 0 ? totalCostPerHead / dailyGainKg : 0;

    return {
      breakdown,
      totalCost,
      totalCostPerHead,
      totalWeight,
      costPerKg,
      costPerEKE,
      costPerCPKg,
      monthlyCost,
      costPerLiterMilk,
      costPerKgGain,
    };
  }, [animalCount, animalProperties, language, localItems, nutrients]);

  if (!economics) {
    return (
      <div className="py-8 text-center text-sm text-[--text-secondary]">
        {t('economics.addFeedsToSee')}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-4 gap-3">
        <SummaryCard
          icon={DollarSign}
          label={t('economics.dailyCost')}
          value={formatCurrency(economics.totalCost)}
          unit={t('units.rub')}
          color="var(--accent)"
        />
        <SummaryCard
          icon={Scale}
          label={t('economics.costPerKg')}
          value={formatNumber(economics.costPerKg, 2)}
          unit={t('units.rubKg')}
        />
        <SummaryCard
          icon={TrendingUp}
          label={t('economics.monthlyCost')}
          value={formatCurrency(economics.monthlyCost)}
          unit={t('units.rub')}
        />
        {animalProperties.milkYieldKg ? (
          <SummaryCard
            icon={Percent}
            label={t('economics.costPerLiterMilk')}
            value={formatNumber(economics.costPerLiterMilk, 2)}
            unit={t('units.rubL')}
          />
        ) : animalProperties.dailyGainG ? (
          <SummaryCard
            icon={Percent}
            label={t('economics.costPerKgGain')}
            value={formatNumber(economics.costPerKgGain, 2)}
            unit={t('units.rubKg')}
          />
        ) : (
          <SummaryCard
            icon={Percent}
            label={t('economics.costPerKg')}
            value={formatNumber(economics.costPerKg, 2)}
            unit={t('units.rubKg')}
          />
        )}
      </div>

      <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-secondary]">
        {t('nutrients.perHead')}: <span className="font-medium text-[--text-primary]">{formatCurrency(economics.totalCostPerHead)}</span>
        {' | '}
        {t('nutrients.forAllHeads')}: <span className="font-medium text-[--text-primary]">{formatCurrency(economics.totalCost)}</span>
      </div>

      <div className="rounded-[--radius-md] bg-[--bg-surface] p-4">
        <h3 className="mb-3 text-xs font-medium uppercase tracking-wide text-[--text-secondary]">
          {t('economics.costEfficiency')}
        </h3>
        <div className="grid grid-cols-2 gap-4">
          <MetricRow
            label={t('economics.costPerEke')}
            value={`${formatNumber(economics.costPerEKE, 2)} ${t('units.rub')}`}
          />
          <MetricRow
            label={t('economics.costPerCp')}
            value={`${formatNumber(economics.costPerCPKg, 2)} ${t('units.rub')}`}
          />
        </div>
      </div>

      <div className="rounded-[--radius-md] bg-[--bg-surface] p-4">
        <h3 className="mb-3 text-xs font-medium uppercase tracking-wide text-[--text-secondary]">
          {t('economics.costBreakdown')}
        </h3>
        <div className="space-y-2">
          {economics.breakdown.map((item) => (
            <CostBreakdownRow key={item.name} item={item} animalCount={animalCount} />
          ))}
        </div>
        <div className="mt-3 flex items-center justify-between border-t border-[--border] pt-3">
          <span className="text-xs font-medium text-[--text-primary]">{t('common.total')}</span>
          <span className="text-sm font-semibold text-[--accent]">
            {formatCurrency(economics.totalCost)} {t('units.rubDay')}
          </span>
        </div>
      </div>

      <div className="rounded-[--radius-md] bg-[--bg-surface] p-4">
        <h3 className="mb-3 text-xs font-medium uppercase tracking-wide text-[--text-secondary]">
          {t('economics.priceTrend')}
        </h3>
        <div className="flex h-24 items-center justify-center text-xs text-[--text-disabled]">
          {t('economics.priceChartSoon')}
        </div>
      </div>
    </div>
  );
}

interface SummaryCardProps {
  icon: typeof DollarSign;
  label: string;
  value: string;
  unit: string;
  color?: string;
}

function SummaryCard({ icon, label, value, unit, color }: SummaryCardProps) {
  return (
    <div className="rounded-[--radius-md] bg-[--bg-surface] p-3">
      <div className="mb-2 flex items-center gap-2">
        <Icon icon={icon} size={14} className="text-[--text-secondary]" />
        <span className="text-[10px] uppercase tracking-wide text-[--text-secondary]">{label}</span>
      </div>
      <div className="flex items-baseline gap-1">
        <span className="text-lg font-semibold" style={{ color: color ?? 'var(--text-primary)' }}>
          {value}
        </span>
        <span className="text-[10px] text-[--text-disabled]">{unit}</span>
      </div>
    </div>
  );
}

function MetricRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between rounded-[--radius-sm] bg-[--bg-base] p-2">
      <span className="text-xs text-[--text-secondary]">{label}</span>
      <span className="text-sm font-medium text-[--text-primary]">{value}</span>
    </div>
  );
}

interface CostBreakdownRowProps {
  item: CostBreakdownItem;
  animalCount: number;
}

function CostBreakdownRow({ item, animalCount }: CostBreakdownRowProps) {
  const groupCost = item.daily_cost * animalCount;

  return (
    <div className="flex items-center gap-2 py-1.5">
      <span className="w-40 truncate text-xs text-[--text-primary]">{item.name}</span>

      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-[--bg-hover]">
        <div
          className="h-full rounded-full bg-[--accent] transition-all duration-300"
          style={{ width: `${Math.min(item.percentage, 100)}%` }}
        />
      </div>

      <span className="w-12 text-right text-xs text-[--text-secondary]">
        {formatNumber(item.percentage, 1)}%
      </span>

      <span className="w-24 text-right text-xs font-medium text-[--text-primary]">
        {formatNumber(groupCost, 2)} ₽
        <span className="block text-[10px] font-normal text-[--text-disabled]">
          {formatNumber(item.daily_cost, 2)} / гол.
        </span>
      </span>
    </div>
  );
}
