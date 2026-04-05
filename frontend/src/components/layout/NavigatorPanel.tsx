import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Home, FileText, DollarSign, Settings } from 'lucide-react';
import { Icon } from '../ui/Icon';
import { cn } from '@/lib/utils';
import { useRationStore } from '@/stores/rationStore';
import { WorkspaceExplorer } from '../workspace/WorkspaceExplorer';
import { NewRationDialog } from '../workspace/NewRationDialog';
import type { RationProject } from '@/types/ration-project';
import { feedsApi } from '@/lib/api';
import { loadGeneratedFeedCatalog, useFeedCatalog } from '@/lib/feed-catalog';

const BOTTOM_NAV = [
  { id: 'dashboard', labelKey: 'nav.dashboard', icon: Home },
  { id: 'norms', labelKey: 'nav.norms', icon: FileText },
  { id: 'prices', labelKey: 'nav.prices', icon: DollarSign },
  { id: 'settings', labelKey: 'nav.settings', icon: Settings },
] as const;

export function NavigatorPanel() {
  const { t } = useTranslation();
  const { activeView, setActiveView, openWorkspaceProject } = useRationStore();
  const [newRationOpen, setNewRationOpen] = useState(false);
  const [newRationFolder, setNewRationFolder] = useState('');
  const [refreshVersion, setRefreshVersion] = useState(0);
  const { feeds: liveCatalog } = useFeedCatalog();

  const handleOpenRation = async (path: string, project: RationProject) => {
    let feedCatalog = liveCatalog;
    try {
      const response = await feedsApi.list({ limit: 5000 });
      if (response.data?.length) {
        feedCatalog = response.data;
      }
    } catch {
      const generatedCatalog = await loadGeneratedFeedCatalog();
      feedCatalog = liveCatalog.length ? liveCatalog : generatedCatalog;
    }

    openWorkspaceProject(path, project, feedCatalog);
  };

  const handleNewRation = (folderPath: string) => {
    setNewRationFolder(folderPath);
    setNewRationOpen(true);
  };

  const handleRationCreated = (path: string, project: RationProject) => {
    void handleOpenRation(path, project);
    setRefreshVersion((value) => value + 1);
  };

  const isBottomSelected = (id: string) => activeView === id;

  return (
    <aside
      className="w-60 flex-shrink-0 border-r flex flex-col overflow-hidden"
      style={{ background: 'var(--bg-sidebar)', borderColor: 'var(--border)' }}
    >
      {/* Workspace file explorer — fills available space */}
      <div className="flex-1 overflow-hidden">
        <WorkspaceExplorer onOpenRation={(path, project) => void handleOpenRation(path, project)} onNewRation={handleNewRation} refreshKey={refreshVersion} />
      </div>

      {/* Bottom navigation: Dashboard, Norms, Prices, Settings */}
      <div className="border-t border-[--border] p-1.5 space-y-0.5">
        {BOTTOM_NAV.map((item) => (
          <button
            key={item.id}
            onClick={() => setActiveView(item.id)}
            className={cn(
              'w-full flex items-center gap-2 px-2 py-1.5 text-xs rounded-[--radius-sm] transition-colors',
              'hover:bg-[--bg-hover]',
              isBottomSelected(item.id) && 'bg-[--bg-active] text-[--accent]',
            )}
          >
            <Icon
              icon={item.icon}
              size={16}
              className={isBottomSelected(item.id) ? 'text-[--accent]' : 'text-[--text-secondary]'}
            />
            <span className="flex-1 text-left truncate">{t(item.labelKey)}</span>
          </button>
        ))}
      </div>

      {/* New Ration Dialog */}
      <NewRationDialog
        open={newRationOpen}
        onClose={() => setNewRationOpen(false)}
        folderPath={newRationFolder}
        onCreated={handleRationCreated}
      />
    </aside>
  );
}
