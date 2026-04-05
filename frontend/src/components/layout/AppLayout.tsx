import { useState } from 'react';
import { Bot, ChevronLeft, ChevronRight } from 'lucide-react';
import { TitleBar } from './TitleBar';
import { NavigatorPanel } from './NavigatorPanel';
import { StatusBar } from './StatusBar';
import { MainWorkspace } from './MainWorkspace';
import { FeedLibraryPanel } from '../feeds/FeedLibraryPanel';
import { AgentChat } from '../agent/AgentChat';
import { Icon } from '../ui/Icon';
import { cn } from '@/lib/utils';
import { isRationWorkspaceView, useRationStore } from '@/stores/rationStore';

export function AppLayout() {
  const [showAgent, setShowAgent] = useState(false);
  const { activeView } = useRationStore();
  const showFeedLibrary = isRationWorkspaceView(activeView);

  return (
    <div
      className="flex flex-col h-screen overflow-hidden"
      style={{ background: 'var(--bg-base)', color: 'var(--text-primary)' }}
    >
      <TitleBar />
      <div className="relative flex flex-1 overflow-hidden">
        <NavigatorPanel />
        <MainWorkspace />
        {showFeedLibrary ? <FeedLibraryPanel /> : null}

        <button
          onClick={() => setShowAgent((current) => !current)}
          className={cn(
            'absolute right-0 top-1/2 -translate-y-1/2 z-10',
            'flex items-center gap-1 px-1.5 py-3 rounded-l-[--radius-md]',
            'bg-[--bg-surface] border border-r-0 border-[--border]',
            'hover:bg-[--bg-hover] transition-colors',
            showAgent && 'right-80'
          )}
          title={showAgent ? 'Hide AI Assistant' : 'Show AI Assistant'}
        >
          <Icon icon={Bot} size={16} className={cn(showAgent ? 'text-[--accent]' : 'text-[--text-secondary]')} />
          <Icon icon={showAgent ? ChevronRight : ChevronLeft} size={12} className="text-[--text-disabled]" />
        </button>

        <aside
          className={cn(
            'w-80 flex-shrink-0 border-l border-[--border] bg-[--bg-sidebar]',
            'transition-all duration-200',
            showAgent ? 'translate-x-0' : 'translate-x-full w-0 overflow-hidden border-0'
          )}
        >
          {showAgent ? <AgentChat /> : null}
        </aside>
      </div>
      <StatusBar />
    </div>
  );
}
