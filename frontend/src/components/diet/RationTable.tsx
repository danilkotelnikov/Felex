import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Trash2, Lock, Unlock, GripVertical } from 'lucide-react';
import type { TFunction } from 'i18next';
import { Badge } from '../ui/Badge';
import { Icon } from '../ui/Icon';
import { Input } from '../ui/Input';
import { getFeedDisplayName, getFeedSecondaryName, resolveFeedLanguage } from '@/lib/feed-display';
import { localizeOptimizationReason, matchAutoAddedFeed } from '@/lib/optimization-feedback';
import { cn, formatNumber } from '@/lib/utils';
import { useRationStore, calculateLocalNutrients } from '@/stores/rationStore';
import type { Feed } from '@/types/feed';
import type { AutoAddedFeed } from '@/types/ration';

interface LocalRationItem {
  id: string;
  feed: Feed;
  amount_kg: number;
  is_locked: boolean;
}

export function RationTable() {
  const { t, i18n } = useTranslation();
  const { localItems, setLocalItems, updateAmount, removeFeed, toggleLock, setNutrients, animalCount, optimizationFeedback } = useRationStore();
  const autoAddedFeeds = optimizationFeedback && !optimizationFeedback.isStale
    ? optimizationFeedback.solution.auto_added_feeds ?? []
    : [];
  const language = resolveFeedLanguage(i18n.resolvedLanguage);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  // Recalculate nutrients when items change
  const handleAmountChange = useCallback(
    (localId: string, value: string) => {
      const amount = parseFloat(value) || 0;
      updateAmount(localId, amount);

      // Recalculate nutrients
      const items = localItems.map((i) =>
        i.id === localId ? { ...i, amount_kg: amount } : i
      );
      setNutrients(calculateLocalNutrients(items));
    },
    [localItems, updateAmount, setNutrients]
  );

  const handleRemove = useCallback(
    (localId: string) => {
      removeFeed(localId);

      // Recalculate nutrients
      const remaining = localItems.filter((i) => i.id !== localId);
      setNutrients(calculateLocalNutrients(remaining));
    },
    [localItems, removeFeed, setNutrients]
  );

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;

      if (over && active.id !== over.id) {
        const oldIndex = localItems.findIndex((item) => item.id === active.id);
        const newIndex = localItems.findIndex((item) => item.id === over.id);

        const newItems = arrayMove(localItems, oldIndex, newIndex);
        setLocalItems(newItems);
      }
    },
    [localItems, setLocalItems]
  );

  // Calculate totals
  const totals = localItems.reduce(
    (acc, item) => {
      const dm_pct = item.feed.dry_matter ?? 86;
      const price_per_kg = (item.feed.price_per_ton ?? 0) / 1000;
      const dm_kg = item.amount_kg * dm_pct / 100;
      const cost = item.amount_kg * price_per_kg;

      return {
        weight: acc.weight + item.amount_kg,
        dm: acc.dm + dm_kg,
        cost: acc.cost + cost,
      };
    },
    { weight: 0, dm: 0, cost: 0 }
  );
  const groupWeight = totals.weight * animalCount;
  const groupCost = totals.cost * animalCount;

  if (localItems.length === 0) {
    return (
      <div className="border border-dashed border-[--border] rounded-[--radius-md] p-8 text-center">
        <p className="text-sm text-[--text-secondary] mb-2">
          {t('workspace.noFeeds')}
        </p>
        <p className="text-xs text-[--text-disabled]">
          {t('workspace.dragFeedsHint')}
        </p>
      </div>
    );
  }

  return (
    <div className="border border-[--border] rounded-[--radius-md] overflow-hidden">
      <table className="w-full text-xs">
        <thead className="bg-[--bg-surface]">
          <tr>
            <th className="w-6 px-2 py-2"></th>
            <th className="text-left px-3 py-2 font-medium text-[--text-secondary]">
              {t('ration.feed')}
            </th>
            <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-24">
              {t('ration.kgDay')}
            </th>
            <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-28">
              {t('ration.kgGroupDay')}
            </th>
            <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-20">
              {t('ration.percentDm')}
            </th>
            <th className="text-right px-3 py-2 font-medium text-[--text-secondary] w-28">
              {t('ration.rubGroupDay')}
            </th>
            <th className="w-16 px-2"></th>
          </tr>
        </thead>
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <SortableContext
            items={localItems.map((item) => item.id)}
            strategy={verticalListSortingStrategy}
          >
            <tbody>
              {localItems.map((item) => (
                <SortableRow
                  key={item.id}
                  item={item}
                  totals={totals}
                  animalCount={animalCount}
                  language={language}
                  autoAddedFeed={matchAutoAddedFeed(item.feed, autoAddedFeeds)}
                  onAmountChange={handleAmountChange}
                  onRemove={handleRemove}
                  onToggleLock={toggleLock}
                  t={t}
                />
              ))}
            </tbody>
          </SortableContext>
        </DndContext>
        <tfoot className="bg-[--bg-surface] font-medium">
          <tr className="border-t border-[--border]">
            <td></td>
            <td className="px-3 py-2 text-[--text-primary]">{t('common.total')}</td>
            <td className="px-3 py-2 text-right text-[--text-primary]">
              {formatNumber(totals.weight, 1)}
            </td>
            <td className="px-3 py-2 text-right text-[--text-primary]">
              {formatNumber(groupWeight, 1)}
            </td>
            <td className="px-3 py-2 text-right text-[--text-secondary]">100%</td>
            <td className="px-3 py-2 text-right text-[--accent] font-semibold">
              {formatNumber(groupCost, 2)}
            </td>
            <td></td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}

interface SortableRowProps {
  item: LocalRationItem;
  totals: { weight: number; dm: number; cost: number };
  animalCount: number;
  language: 'ru' | 'en';
  autoAddedFeed: AutoAddedFeed | null;
  onAmountChange: (id: string, value: string) => void;
  onRemove: (id: string) => void;
  onToggleLock: (id: string) => void;
  t: TFunction;
}

function SortableRow({
  item,
  totals,
  animalCount,
  language,
  autoAddedFeed,
  onAmountChange,
  onRemove,
  onToggleLock,
  t,
}: SortableRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: item.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const dm_pct = item.feed.dry_matter ?? 86;
  const dm_kg = item.amount_kg * dm_pct / 100;
  const dm_pct_of_total = totals.dm > 0 ? (dm_kg / totals.dm) * 100 : 0;
  const price_per_kg = (item.feed.price_per_ton ?? 0) / 1000;
  const cost = item.amount_kg * price_per_kg;
  const groupAmount = item.amount_kg * animalCount;
  const groupCost = cost * animalCount;
  const displayName = getFeedDisplayName(item.feed, language);
  const secondaryName = getFeedSecondaryName(item.feed, language);

  return (
    <tr
      ref={setNodeRef}
      style={style}
      className={cn(
        'border-t border-[--border]',
        'hover:bg-[--bg-hover] transition-colors',
        isDragging && 'bg-[--bg-active]'
      )}
    >
      <td
        className="px-2 py-1.5 text-center cursor-grab"
        {...attributes}
        {...listeners}
      >
        <Icon
          icon={GripVertical}
          size={14}
          className="text-[--text-disabled]"
        />
      </td>
      <td className="px-3 py-1.5">
        <div>
          <div>
            <span className="text-[--text-primary]">{displayName}</span>
            {secondaryName && (
              <span className="text-[--text-disabled] ml-1">
                ({secondaryName})
              </span>
            )}
          </div>
          {autoAddedFeed ? (
            <div className="mt-1 space-y-1">
              <Badge variant="info" className="px-1.5 py-0 text-[10px]">
                {t('ration.autoAddedBadge')}
              </Badge>
              {autoAddedFeed.reasons.length ? (
                <div className="text-[10px] leading-4 text-[--text-secondary]">
                  {autoAddedFeed.reasons
                    .slice(0, 2)
                    .map((reason) => localizeOptimizationReason(reason, t))
                    .join(' ')}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </td>
      <td className="px-3 py-1.5">
        <Input
          type="number"
          value={item.amount_kg}
          onChange={(e) => onAmountChange(item.id, e.target.value)}
          disabled={item.is_locked}
          className="w-20 h-6 text-right text-xs"
          step="0.1"
          min="0"
        />
      </td>
      <td className="px-3 py-1.5 text-right text-[--text-secondary]">
        {formatNumber(groupAmount, 2)}
      </td>
      <td className="px-3 py-1.5 text-right text-[--text-secondary]">
        {formatNumber(dm_pct_of_total, 1)}%
      </td>
      <td className="px-3 py-1.5 text-right text-[--text-secondary]">
        <div>{formatNumber(groupCost, 2)}</div>
        <div className="text-[10px] text-[--text-disabled]">
          {formatNumber(cost, 2)} / {t('nutrients.perHead')}
        </div>
      </td>
      <td className="px-2 py-1.5">
        <div className="flex items-center gap-1 justify-end">
          <button
            onClick={() => onToggleLock(item.id)}
            className={cn(
              'p-1 rounded hover:bg-[--bg-active] transition-colors',
              item.is_locked ? 'text-[--accent]' : 'text-[--text-disabled]'
            )}
            title={item.is_locked ? t('ration.unlockAmount') : t('ration.lockAmount')}
          >
            <Icon icon={item.is_locked ? Lock : Unlock} size={14} />
          </button>
          <button
            onClick={() => onRemove(item.id)}
            className="p-1 rounded text-[--text-disabled] hover:text-[--status-error] hover:bg-[--status-error-bg] transition-colors"
            title={t('ration.removeFeed')}
          >
            <Icon icon={Trash2} size={14} />
          </button>
        </div>
      </td>
    </tr>
  );
}
