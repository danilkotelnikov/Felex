import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';
import toast from 'react-hot-toast';
import { Bell, Bot, Database, Globe, Moon, Save, Sun } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { appApi, feedsApi } from '@/lib/api';
import {
  localizeCriticalContext,
  localizeCriticalNutrientKey,
} from '@/lib/feed-critical-audit';
import { useTheme } from '@/lib/theme';
import { cn } from '@/lib/utils';
import {
  DEFAULT_NOTIFICATION_PREFERENCES,
  isNotificationEnabled,
  loadNotificationPreferences,
  saveNotificationPreferences,
  type NotificationPreferences,
} from '@/lib/preferences';
import { useAgentStore } from '@/stores/agentStore';

type SettingsSection = 'appearance' | 'agent' | 'database' | 'notifications';

const CONTEXT_OPTIONS = [1024, 2048, 4096, 8192, 16384] as const;

interface DatabaseMetaState {
  version: string;
  databasePath: string;
  workspaceRoot: string;
  feedCount: number;
  lastSyncAt: string | null;
  catalogQuality: {
    sourceCounts: {
      normalized: number;
      curated: number;
      custom: number;
      imported: number;
    };
    translationCounts: {
      ready: number;
      sourceOnly: number;
    };
    profileCounts: {
      complete: number;
      partial: number;
      limited: number;
    };
    pricedFeedCount: number;
    unpricedFeedCount: number;
    benchmarkCriticalContexts: Array<{
      id: string;
      species: string;
      stageContext: string;
      auditedFeedCount: number;
      coverageCounts: {
        complete: number;
        partial: number;
        limited: number;
      };
      topMissingKeys: Array<{
        key: import('@/types/feed').FeedCriticalNutrientKey;
        count: number;
      }>;
    }>;
  };
}

export function SettingsPanel() {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsSection>('appearance');

  const sections = [
    { id: 'appearance' as const, labelKey: 'settings.appearance', icon: Sun },
    { id: 'agent' as const, labelKey: 'settings.aiAgent', icon: Bot },
    { id: 'database' as const, labelKey: 'settings.database', icon: Database },
    { id: 'notifications' as const, labelKey: 'settings.notifications', icon: Bell },
  ];

  return (
    <div className="flex h-full w-full min-w-0">
      <div className="w-48 flex-shrink-0 border-r border-[--border] bg-[--bg-sidebar] p-2">
        {sections.map((section) => (
          <button
            key={section.id}
            onClick={() => setActiveSection(section.id)}
            className={cn(
              'w-full rounded-[--radius-sm] px-3 py-2 text-xs transition-colors',
              'flex items-center gap-2',
              activeSection === section.id
                ? 'bg-[--bg-active] text-[--accent]'
                : 'text-[--text-secondary] hover:bg-[--bg-hover]'
            )}
          >
            <Icon icon={section.icon} size={14} />
            {t(section.labelKey)}
          </button>
        ))}
      </div>

      <div className="flex-1 min-w-0 overflow-y-auto bg-[--bg-base] p-6">
        {activeSection === 'appearance' ? <AppearanceSettings /> : null}
        {activeSection === 'agent' ? <AgentSettings /> : null}
        {activeSection === 'database' ? <DatabaseSettings /> : null}
        {activeSection === 'notifications' ? <NotificationSettings /> : null}
      </div>
    </div>
  );
}

function AppearanceSettings() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useTheme();

  return (
    <div className="max-w-lg space-y-6">
      <div>
        <h2 className="mb-1 text-sm font-medium text-[--text-primary]">{t('settings.appearance')}</h2>
        <p className="text-xs text-[--text-secondary]">{t('settings.appearanceDescription')}</p>
      </div>

      <div className="space-y-4">
        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{t('settings.theme')}</label>
          <div className="flex gap-2">
            <ThemeButton icon={Sun} label={t('settings.light')} active={theme === 'light'} onClick={() => setTheme('light')} />
            <ThemeButton icon={Moon} label={t('settings.dark')} active={theme === 'dark'} onClick={() => setTheme('dark')} />
          </div>
        </div>

        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{t('settings.language')}</label>
          <select
            value={i18n.language}
            onChange={(event) => i18n.changeLanguage(event.target.value)}
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-primary]"
          >
            <option value="ru">Русский</option>
            <option value="en">English</option>
          </select>
        </div>

        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{t('settings.numberFormat')}</label>
          <select className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-primary]">
            <option value="ru">{t('settings.numberFormatRu')}</option>
            <option value="en">{t('settings.numberFormatEn')}</option>
          </select>
        </div>
      </div>
    </div>
  );
}

function AgentSettings() {
  const { t, i18n } = useTranslation();
  const { settings, setSettings, status, statusLoading, reloadAgent } = useAgentStore();
  const contextLabel = i18n.language === 'ru' ? 'Длина контекста' : 'Context length';
  const contextHint = i18n.language === 'ru'
    ? 'Для крупных моделей вроде Qwen 3.5 9B лучше начинать с 4096-8192 токенов, чтобы снизить риск ошибок Ollama.'
    : 'For larger models like Qwen 3.5 9B, start with 4096-8192 tokens to reduce Ollama errors.';

  const handleSave = async () => {
    await reloadAgent({
      model: settings.model,
      backend: 'ollama',
      webEnabled: settings.webSearch,
      contextSize: settings.contextSize,
    });
  };

  return (
    <div className="max-w-lg space-y-6">
      <div>
        <h2 className="mb-1 text-sm font-medium text-[--text-primary]">{t('settings.aiAgent')}</h2>
        <p className="text-xs text-[--text-secondary]">{t('settings.agentDescription')}</p>
      </div>

      <div
        className={cn(
          'rounded-[--radius-md] border px-3 py-2 text-xs',
          statusLoading
            ? 'border-[--border] bg-[--bg-surface] text-[--text-secondary]'
            : status.modelLoaded
              ? 'border-green-500/30 bg-green-500/10 text-green-700'
              : 'border-yellow-500/30 bg-yellow-500/10 text-yellow-700'
        )}
      >
        {statusLoading
          ? t('agent.connecting')
          : status.modelLoaded
            ? `${t('settings.connected')}: ${status.modelName} | ${status.contextSize}`
            : t('agent.modelNotLoaded')}
      </div>

      <div className="space-y-4">
        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{t('settings.model')}</label>
          <select
            value={settings.model}
            onChange={(event) => setSettings({ model: event.target.value as 'qwen3.5:4b' | 'qwen3.5:9b' })}
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-primary]"
          >
            <option value="qwen3.5:4b">Qwen 3.5 - 4B</option>
            <option value="qwen3.5:9b">Qwen 3.5 - 9B</option>
          </select>
        </div>

        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{t('settings.backend')}</label>
          <div className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-primary]">
            Ollama
          </div>
        </div>

        <div>
          <label className="mb-2 block text-xs font-medium text-[--text-primary]">{contextLabel}</label>
          <select
            value={settings.contextSize}
            onChange={(event) =>
              setSettings({ contextSize: Number(event.target.value) as (typeof CONTEXT_OPTIONS)[number] })
            }
            className="w-full rounded-[--radius-md] border border-[--border] bg-[--bg-surface] px-3 py-2 text-xs text-[--text-primary]"
          >
            {CONTEXT_OPTIONS.map((value) => (
              <option key={value} value={value}>
                {value.toLocaleString(i18n.language === 'ru' ? 'ru-RU' : 'en-US')} tokens
              </option>
            ))}
          </select>
          <p className="mt-1 text-[10px] text-[--text-secondary]">{contextHint}</p>
        </div>

        <ToggleRow
          title={t('settings.webSearch')}
          description={t('settings.webSearchDescription')}
          checked={settings.webSearch}
          onChange={(checked) => setSettings({ webSearch: checked })}
        />

        <ToggleRow
          title={t('settings.autoSuggest')}
          description={t('settings.autoSuggestDescription')}
          checked={settings.autoSuggest}
          onChange={(checked) => setSettings({ autoSuggest: checked })}
        />

        <Button size="sm" onClick={handleSave} disabled={statusLoading}>
          <Icon icon={Save} size={14} className="mr-1.5" />
          {t('settings.saveSettings')}
        </Button>
      </div>
    </div>
  );
}

function DatabaseSettings() {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const [meta, setMeta] = useState<DatabaseMetaState | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);

  const loadMeta = async () => {
    setLoading(true);
    try {
      const response = await appApi.getMeta();
      setMeta({
        version: response.data.version,
        databasePath: response.data.database_path,
        workspaceRoot: response.data.workspace_root,
        feedCount: response.data.feed_count,
        lastSyncAt: response.data.last_sync_at ?? null,
        catalogQuality: {
          sourceCounts: {
            normalized: response.data.catalog_quality.source_counts.normalized,
            curated: response.data.catalog_quality.source_counts.curated,
            custom: response.data.catalog_quality.source_counts.custom,
            imported: response.data.catalog_quality.source_counts.imported,
          },
          translationCounts: {
            ready: response.data.catalog_quality.translation_counts.ready,
            sourceOnly: response.data.catalog_quality.translation_counts.source_only,
          },
          profileCounts: {
            complete: response.data.catalog_quality.profile_counts.complete,
            partial: response.data.catalog_quality.profile_counts.partial,
            limited: response.data.catalog_quality.profile_counts.limited,
          },
          pricedFeedCount: response.data.catalog_quality.priced_feed_count,
          unpricedFeedCount: response.data.catalog_quality.unpriced_feed_count,
          benchmarkCriticalContexts: response.data.catalog_quality.benchmark_critical_contexts.map((context) => ({
            id: context.id,
            species: context.species,
            stageContext: context.stage_context,
            auditedFeedCount: context.audited_feed_count,
            coverageCounts: {
              complete: context.coverage_counts.complete,
              partial: context.coverage_counts.partial,
              limited: context.coverage_counts.limited,
            },
            topMissingKeys: context.top_missing_keys.map((item) => ({
              key: item.key,
              count: item.count,
            })),
          })),
        },
      });
    } catch (error) {
      console.error('Failed to load app metadata:', error);
      setMeta(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadMeta();
  }, []);

  const lastSyncLabel = useMemo(() => {
    if (!meta?.lastSyncAt) {
      return t('settings.notSynced');
    }
    return formatRelativeTime(meta.lastSyncAt, i18n.language);
  }, [i18n.language, meta?.lastSyncAt, t]);

  const handleSync = async () => {
    setSyncing(true);
    try {
      const result = await feedsApi.sync();
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['feedCatalog'] }),
        queryClient.invalidateQueries({ queryKey: ['feedDetail'] }),
        queryClient.invalidateQueries({ queryKey: ['feedPrice'] }),
      ]);
      await loadMeta();
      if (isNotificationEnabled('priceSync')) {
        toast.success(
          t('settings.syncSuccess', {
            feeds: result.data.feeds_total,
            prices: result.data.prices_updated,
          }),
        );
      }
    } catch (error) {
      console.error('Failed to sync feeds:', error);
      if (isNotificationEnabled('priceSync')) {
        toast.error(t('settings.syncError'));
      }
    } finally {
      setSyncing(false);
    }
  };

  const feedCount = meta?.feedCount ?? 0;
  const translationReady = meta?.catalogQuality.translationCounts.ready ?? 0;
  const translationSourceOnly = meta?.catalogQuality.translationCounts.sourceOnly ?? 0;
  const profileComplete = meta?.catalogQuality.profileCounts.complete ?? 0;
  const pricedFeedCount = meta?.catalogQuality.pricedFeedCount ?? 0;
  const profilePartial = meta?.catalogQuality.profileCounts.partial ?? 0;
  const profileLimited = meta?.catalogQuality.profileCounts.limited ?? 0;
  const unpricedFeedCount = meta?.catalogQuality.unpricedFeedCount ?? 0;
  const benchmarkCriticalContexts = meta?.catalogQuality.benchmarkCriticalContexts ?? [];
  const criticalContextsWithOpenGaps = benchmarkCriticalContexts.filter(
    (context) => context.topMissingKeys.length > 0 || context.coverageCounts.limited > 0,
  ).length;
  const closureIssues = [
    translationSourceOnly > 0
      ? t('settings.closureIssueTranslations', { count: translationSourceOnly })
      : null,
    profileLimited > 0
      ? t('settings.closureIssueProfiles', { count: profileLimited })
      : null,
    unpricedFeedCount > 0
      ? t('settings.closureIssuePrices', { count: unpricedFeedCount })
      : null,
    criticalContextsWithOpenGaps > 0
      ? t('settings.closureIssueCriticalContexts', { count: criticalContextsWithOpenGaps })
      : null,
  ].filter(Boolean) as string[];

  const databaseStatus = loading
    ? t('common.loading')
    : meta
      ? t('settings.connected')
      : t('common.error');
  const databaseStatusTone = loading ? 'muted' : meta ? 'good' : 'danger';

  return (
    <div className="max-w-2xl space-y-6">
      <div>
        <h2 className="mb-1 text-sm font-medium text-[--text-primary]">{t('settings.database')}</h2>
        <p className="text-xs text-[--text-secondary]">{t('settings.databaseDescription')}</p>
      </div>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label={t('settings.feedRecords')}
          value={formatCount(feedCount, i18n.language)}
          hint={t('settings.feedRecordsHint')}
          tone="neutral"
        />
        <MetricCard
          label={t('settings.translationCoverage')}
          value={formatCoverage(translationReady, feedCount, i18n.language)}
          hint={t('settings.translationCoverageHint')}
          tone={coverageTone(translationReady, feedCount)}
        />
        <MetricCard
          label={t('settings.completeProfiles')}
          value={formatCoverage(profileComplete, feedCount, i18n.language)}
          hint={t('settings.completeProfilesHint')}
          tone={coverageTone(profileComplete, feedCount)}
        />
        <MetricCard
          label={t('settings.priceCoverage')}
          value={formatCoverage(pricedFeedCount, feedCount, i18n.language)}
          hint={t('settings.priceCoverageHint')}
          tone={coverageTone(pricedFeedCount, feedCount)}
        />
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <InfoCard
          title={t('settings.feedDatabase')}
          status={databaseStatus}
          statusTone={databaseStatusTone}
          loading={loading}
          rows={[
            { label: t('settings.path'), value: meta?.databasePath ?? '—' },
            { label: t('settings.feeds'), value: meta ? `${formatCount(meta.feedCount, i18n.language)} ${t('settings.entries')}` : '—' },
            { label: t('settings.lastSync'), value: lastSyncLabel },
          ]}
        />
        <InfoCard
          title={t('settings.workspaceRoot')}
          status={`v${meta?.version ?? '1.0.0'}`}
          statusTone="muted"
          loading={loading}
          rows={[
            { label: t('settings.workspaceRoot'), value: meta?.workspaceRoot ?? '—' },
            { label: t('settings.applicationVersion'), value: meta?.version ?? '1.0.0' },
          ]}
        />
      </div>

      <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
        <div className="text-xs font-medium text-[--text-primary]">{t('settings.dataAuthority')}</div>
        <p className="mt-1 text-xs text-[--text-secondary]">{t('settings.dataAuthorityBody')}</p>
        <p className="mt-2 text-[10px] text-[--text-secondary]">{t('settings.dataAuthorityHint')}</p>
      </div>

      <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="text-xs font-medium text-[--text-primary]">{t('settings.closureGateTitle')}</div>
            <p className="mt-1 text-xs text-[--text-secondary]">
              {closureIssues.length ? t('settings.closureGateOpen') : t('settings.closureGateReady')}
            </p>
          </div>
          <div className={`rounded-full px-2 py-1 text-[10px] ${
            closureIssues.length
              ? 'bg-amber-500/10 text-amber-700'
              : 'bg-green-500/10 text-green-700'
          }`}>
            {closureIssues.length ? t('settings.closureGateNeedsWork') : t('settings.closureGateAligned')}
          </div>
        </div>
        {closureIssues.length ? (
          <div className="mt-3 space-y-1 text-xs text-[--text-secondary]">
            {closureIssues.map((item) => (
              <div key={item}>{item}</div>
            ))}
          </div>
        ) : null}
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
          <div className="mb-3 text-xs font-medium text-[--text-primary]">{t('settings.catalogCoverage')}</div>
          <div className="space-y-3">
            <CoverageRow
              label={t('settings.translationCoverage')}
              count={translationReady}
              total={feedCount}
              language={i18n.language}
            />
            <CoverageRow
              label={t('settings.completeProfiles')}
              count={profileComplete}
              total={feedCount}
              language={i18n.language}
            />
            <CoverageRow
              label={t('settings.partialProfiles')}
              count={profilePartial}
              total={feedCount}
              language={i18n.language}
            />
            <CoverageRow
              label={t('settings.priceCoverage')}
              count={pricedFeedCount}
              total={feedCount}
              language={i18n.language}
            />
          </div>
        </div>

        <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
          <div className="mb-3 text-xs font-medium text-[--text-primary]">{t('settings.catalogComposition')}</div>
          <div className="space-y-2 text-xs text-[--text-secondary]">
            <SummaryRow
              label={t('feedLibrary.source.normalized')}
              value={formatCount(meta?.catalogQuality.sourceCounts.normalized ?? 0, i18n.language)}
            />
            <SummaryRow
              label={t('feedLibrary.source.curated')}
              value={formatCount(meta?.catalogQuality.sourceCounts.curated ?? 0, i18n.language)}
            />
            <SummaryRow
              label={t('feedLibrary.source.custom')}
              value={formatCount(meta?.catalogQuality.sourceCounts.custom ?? 0, i18n.language)}
            />
            <SummaryRow
              label={t('feedLibrary.source.imported')}
              value={formatCount(meta?.catalogQuality.sourceCounts.imported ?? 0, i18n.language)}
            />
            <div className="my-2 border-t border-[--border]" />
            <SummaryRow
              label={t('feedLibrary.profile.complete')}
              value={formatCount(profileComplete, i18n.language)}
            />
            <SummaryRow
              label={t('feedLibrary.profile.partial')}
              value={formatCount(profilePartial, i18n.language)}
            />
            <SummaryRow
              label={t('feedLibrary.profile.limited')}
              value={formatCount(profileLimited, i18n.language)}
            />
            <SummaryRow
              label={t('settings.withoutPrice')}
              value={formatCount(meta?.catalogQuality.unpricedFeedCount ?? 0, i18n.language)}
            />
          </div>
        </div>
      </div>

      <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
        <div className="text-xs font-medium text-[--text-primary]">{t('settings.criticalCoverageTitle')}</div>
        <p className="mt-1 text-xs text-[--text-secondary]">{t('settings.criticalCoverageBody')}</p>
        <div className="mt-4 grid gap-3 xl:grid-cols-2">
          {benchmarkCriticalContexts.map((context) => (
            <div
              key={context.id}
              className="rounded-[--radius-md] border border-[--border] bg-[--bg-base] p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="text-xs font-medium text-[--text-primary]">
                    {localizeCriticalContext(context.id, t)}
                  </div>
                  <div className="mt-1 text-[10px] text-[--text-secondary]">
                    {t('settings.criticalAuditedFeeds', {
                      count: context.auditedFeedCount,
                    })}
                  </div>
                </div>
                <div className="text-[10px] text-[--text-secondary]">
                  {formatCoverage(
                    context.coverageCounts.complete,
                    context.auditedFeedCount,
                    i18n.language,
                  )}
                </div>
              </div>
              <div className="mt-3 space-y-3">
                <CoverageRow
                  label={t('feedLibrary.profile.complete')}
                  count={context.coverageCounts.complete}
                  total={context.auditedFeedCount}
                  language={i18n.language}
                />
                <CoverageRow
                  label={t('feedLibrary.profile.partial')}
                  count={context.coverageCounts.partial}
                  total={context.auditedFeedCount}
                  language={i18n.language}
                />
                <CoverageRow
                  label={t('feedLibrary.profile.limited')}
                  count={context.coverageCounts.limited}
                  total={context.auditedFeedCount}
                  language={i18n.language}
                />
              </div>
              <div className="mt-4 text-[10px] uppercase tracking-wide text-[--text-secondary]">
                {t('settings.criticalTopMissingTitle')}
              </div>
              {context.topMissingKeys.length ? (
                <div className="mt-2 space-y-1 text-xs text-[--text-secondary]">
                  {context.topMissingKeys.map((item) => (
                    <div key={`${context.id}-${item.key}`} className="flex items-center justify-between gap-3">
                      <span>{localizeCriticalNutrientKey(item.key, t, i18n.resolvedLanguage)}</span>
                      <span className="font-medium text-[--text-primary]">
                        {formatCount(item.count, i18n.language)}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="mt-2 text-xs text-[--text-secondary]">
                  {t('settings.criticalNoGaps')}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="flex gap-2">
        <Button variant="outline" size="sm" onClick={handleSync} disabled={syncing}>
          <Icon icon={Globe} size={14} className="mr-1.5" />
          {syncing ? t('prices.fetching') : t('settings.syncFromWeb')}
        </Button>
        <Button variant="outline" size="sm" onClick={() => void loadMeta()}>
          {t('common.refresh')}
        </Button>
      </div>
    </div>
  );
}

function NotificationSettings() {
  const { t } = useTranslation();
  const [preferences, setPreferences] = useState<NotificationPreferences>(DEFAULT_NOTIFICATION_PREFERENCES);

  useEffect(() => {
    setPreferences(loadNotificationPreferences());
  }, []);

  const updatePreference = (key: keyof NotificationPreferences, value: boolean) => {
    const next = {
      ...preferences,
      [key]: value,
    };
    setPreferences(next);
    saveNotificationPreferences(next);
  };

  return (
    <div className="max-w-lg space-y-6">
      <div>
        <h2 className="mb-1 text-sm font-medium text-[--text-primary]">{t('settings.notifications')}</h2>
        <p className="text-xs text-[--text-secondary]">{t('settings.notificationsDescription')}</p>
      </div>

      <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-3 text-xs text-[--text-secondary]">
        {t('settings.notificationsRuntimeNote')}
      </div>

      <div className="space-y-4">
        <ToggleRow
          title={t('settings.nutrientWarnings')}
          description={t('settings.nutrientWarningsDescription')}
          checked={preferences.nutrientWarnings}
          onChange={(checked) => updatePreference('nutrientWarnings', checked)}
        />

        <ToggleRow
          title={t('settings.priceSyncNotifications')}
          description={t('settings.priceSyncNotificationsDescription')}
          checked={preferences.priceSync}
          onChange={(checked) => updatePreference('priceSync', checked)}
        />

        <ToggleRow
          title={t('settings.exportNotifications')}
          description={t('settings.exportNotificationsDescription')}
          checked={preferences.exportReady}
          onChange={(checked) => updatePreference('exportReady', checked)}
        />

        <ToggleRow
          title={t('settings.agentNotifications')}
          description={t('settings.agentNotificationsDescription')}
          checked={preferences.agentStatus}
          onChange={(checked) => updatePreference('agentStatus', checked)}
        />
      </div>
    </div>
  );
}

function ToggleRow({
  title,
  description,
  checked,
  onChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between py-2">
      <div>
        <div className="text-xs font-medium text-[--text-primary]">{title}</div>
        <div className="text-[10px] text-[--text-secondary]">{description}</div>
      </div>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="rounded"
      />
    </div>
  );
}

function InfoCard({
  title,
  status,
  rows,
  loading,
  statusTone,
}: {
  title: string;
  status: string;
  rows: Array<{ label: string; value: string }>;
  loading?: boolean;
  statusTone?: 'good' | 'muted' | 'danger';
}) {
  const toneClass =
    statusTone === 'good'
      ? 'text-[--status-ok]'
      : statusTone === 'danger'
        ? 'text-[--status-error]'
        : 'text-[--text-secondary]';

  return (
    <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-[--text-primary]">{title}</span>
        <span className={`text-[10px] ${toneClass}`}>{status}</span>
      </div>
      <div className="space-y-1 text-[10px] text-[--text-secondary]">
        {loading ? (
          <div>…</div>
        ) : (
          rows.map((row) => (
            <div key={row.label}>
              {row.label}: {row.value}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function MetricCard({
  label,
  value,
  hint,
  tone,
}: {
  label: string;
  value: string;
  hint: string;
  tone: 'neutral' | 'good' | 'caution';
}) {
  const toneClass =
    tone === 'good'
      ? 'text-[--status-ok]'
      : tone === 'caution'
        ? 'text-[--status-warn]'
        : 'text-[--text-primary]';

  return (
    <div className="rounded-[--radius-md] border border-[--border] bg-[--bg-surface] p-4">
      <div className="text-[10px] uppercase tracking-wide text-[--text-secondary]">{label}</div>
      <div className={`mt-1 text-lg font-semibold ${toneClass}`}>{value}</div>
      <div className="mt-1 text-[10px] text-[--text-secondary]">{hint}</div>
    </div>
  );
}

function CoverageRow({
  label,
  count,
  total,
  language,
}: {
  label: string;
  count: number;
  total: number;
  language: string;
}) {
  const ratio = total > 0 ? count / total : 0;
  const toneClass =
    ratio >= 0.8 ? 'bg-[--status-ok]' : ratio >= 0.4 ? 'bg-[--accent]' : 'bg-[--status-warn]';

  return (
    <div>
      <div className="mb-1 flex items-center justify-between gap-3 text-[10px] text-[--text-secondary]">
        <span>{label}</span>
        <span>{formatCount(count, language)} / {formatCount(total, language)}</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-[--bg-hover]">
        <div className={`h-full rounded-full ${toneClass}`} style={{ width: `${Math.min(ratio * 100, 100)}%` }} />
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span>{label}</span>
      <span className="font-medium text-[--text-primary]">{value}</span>
    </div>
  );
}

interface ThemeButtonProps {
  icon: typeof Moon;
  label: string;
  active: boolean;
  onClick: () => void;
}

function ThemeButton({ icon, label, active, onClick }: ThemeButtonProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex items-center gap-2 rounded-[--radius-md] border px-4 py-2 transition-colors',
        active
          ? 'border-[--accent] bg-[--bg-active] text-[--accent]'
          : 'border-[--border] bg-[--bg-surface] text-[--text-secondary] hover:border-[--text-disabled]'
      )}
    >
      <Icon icon={icon} size={16} />
      <span className="text-xs">{label}</span>
    </button>
  );
}

function formatRelativeTime(
  value: string,
  language: string,
) {
  const target = new Date(value);
  if (Number.isNaN(target.getTime())) {
    return value;
  }

  const diffMs = Math.max(0, Date.now() - target.getTime());
  const diffMinutes = Math.max(1, Math.floor(diffMs / 60_000));
  const diffHours = Math.floor(diffMinutes / 60);

  if (language === 'ru') {
    if (diffHours >= 1) {
      return `${diffHours} ${pluralRu(diffHours, ['час', 'часа', 'часов'])} назад`;
    }
    return `${diffMinutes} ${pluralRu(diffMinutes, ['минуту', 'минуты', 'минут'])} назад`;
  }

  if (diffHours >= 1) {
    return `${diffHours} ${diffHours === 1 ? 'hour' : 'hours'} ago`;
  }
  return `${diffMinutes} ${diffMinutes === 1 ? 'minute' : 'minutes'} ago`;
}

function formatCount(value: number, language: string): string {
  return value.toLocaleString(language === 'ru' ? 'ru-RU' : 'en-US');
}

function formatCoverage(count: number, total: number, language: string): string {
  if (total <= 0) {
    return '—';
  }
  const percent = Math.round((count / total) * 100);
  return `${formatCount(count, language)} (${percent}%)`;
}

function coverageTone(count: number, total: number): 'neutral' | 'good' | 'caution' {
  if (total <= 0) {
    return 'neutral';
  }
  const ratio = count / total;
  if (ratio >= 0.8) {
    return 'good';
  }
  if (ratio >= 0.4) {
    return 'neutral';
  }
  return 'caution';
}

function pluralRu(value: number, forms: [string, string, string]) {
  const mod10 = value % 10;
  const mod100 = value % 100;

  if (mod10 === 1 && mod100 !== 11) {
    return forms[0];
  }
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) {
    return forms[1];
  }
  return forms[2];
}
