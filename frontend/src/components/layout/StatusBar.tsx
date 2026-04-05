import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Bot, CheckCircle2, Loader2, WifiOff } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { useAgentStore } from '@/stores/agentStore';
import { useRationStore } from '@/stores/rationStore';
import { useResolvedNormReference } from '@/lib/resolved-norms';
import { formatNumber } from '@/lib/utils';
import { getNutrientStatus } from '@/lib/nutrient-status';
import { getManagedNormEntries, getNutrientDisplayActual } from '@/lib/nutrient-display';

export function StatusBar() {
  const { t } = useTranslation();
  const { localItems, nutrients, animalProperties, customNorms, activeNormPresetId, animalCount } = useRationStore();
  const { status, isStreaming } = useAgentStore();
  const { norms: currentNorms, resolvedGroupId } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  const dailyCost = localItems.reduce((sum, item) => {
    const pricePerKg = (item.feed.price_per_ton ?? 0) / 1000;
    return sum + item.amount_kg * pricePerKg;
  }, 0) * animalCount;

  const normStats = useMemo(() => {
    if (!nutrients) {
      return null;
    }

    let total = 0;
    let ok = 0;

    for (const [key, norm] of getManagedNormEntries(resolvedGroupId, currentNorms)) {
      const actual = getNutrientDisplayActual(nutrients, resolvedGroupId, key);

      if (actual === undefined || !Number.isFinite(actual)) {
        continue;
      }

      total += 1;
      if (getNutrientStatus(actual, norm.min, norm.max, norm.target) === 'ok') {
        ok += 1;
      }
    }

    return { ok, total };
  }, [currentNorms, nutrients, resolvedGroupId]);

  const normLabel = normStats ? `${normStats.ok}/${normStats.total}` : '—';

  return (
    <footer
      className="flex h-6 items-center justify-between border-t px-3 text-[10px]"
      style={{
        background: 'var(--bg-elevated)',
        borderColor: 'var(--border)',
        color: 'var(--text-secondary)',
      }}
    >
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-1">
          <Icon icon={CheckCircle2} size={14} className="text-[--status-ok]" />
          <span>{t('statusBar.norms')}: {normLabel}</span>
        </div>
        <span className="text-[--text-disabled]">|</span>
        <span>{t('statusBar.cost')}: {formatNumber(dailyCost, 2)} ₽/{t('common.day')}</span>
        {localItems.length > 0 ? (
          <>
            <span className="text-[--text-disabled]">|</span>
            <span>{t('statusBar.feeds')}: {localItems.length}</span>
          </>
        ) : null}
      </div>

      <div className="flex items-center gap-4">
        <div className="flex items-center gap-1">
          {isStreaming ? (
            <>
              <Icon icon={Loader2} size={14} className="animate-spin text-[--accent]" />
              <span className="text-[--accent]">{t('statusBar.agent')}: {t('statusBar.thinking')}</span>
            </>
          ) : status.modelLoaded ? (
            <>
              <Icon icon={Bot} size={14} className="text-[--status-ok]" />
              <span>{t('statusBar.agent')}: {status.modelName}</span>
            </>
          ) : (
            <>
              <Icon icon={WifiOff} size={14} className="text-[--text-disabled]" />
              <span>{t('statusBar.agent')}: {t('statusBar.offline')}</span>
            </>
          )}
        </div>
        <span>Felex v1.0</span>
      </div>
    </footer>
  );
}
