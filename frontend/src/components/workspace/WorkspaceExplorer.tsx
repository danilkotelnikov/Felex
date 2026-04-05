import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ChevronRight, ChevronDown, FolderPlus, FilePlus, RefreshCw,
  ChevronsDownUp, Folder, FileText, Trash2, Edit2, Download, FolderInput,
} from 'lucide-react';
import toast from 'react-hot-toast';
import { workspaceApi } from '@/lib/workspace-api';
import { downloadRationCSV, downloadRationXLSX } from '@/lib/export';
import { WORKSPACE_REFRESH_EVENT } from '@/lib/workspace-events';
import { useRationStore } from '@/stores/rationStore';
import type { FileNode, RationProject } from '@/types/ration-project';
import { cn } from '@/lib/utils';
import { Icon } from '../ui/Icon';

interface Props {
  onOpenRation: (path: string, project: RationProject) => void;
  onNewRation: (folderPath: string) => void;
  refreshKey?: number;
}

export function WorkspaceExplorer({ onOpenRation, onNewRation, refreshKey = 0 }: Props) {
  const { t } = useTranslation();
  const { currentProjectPath, setCurrentProject } = useRationStore();
  const [tree, setTree] = useState<FileNode | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; node: FileNode } | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await workspaceApi.getTree();
      // Backend returns FileNode[] — wrap into a virtual root
      const children: FileNode[] = Array.isArray(result.data) ? result.data : (result.data as any)?.children ?? [];
      setTree({ name: 'projects', path: '', isDir: true, children });
    } catch {
      setTree({ name: 'projects', path: '', isDir: true, children: [] });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh, refreshKey]);

  useEffect(() => {
    const handleRefresh = () => {
      void refresh();
    };

    window.addEventListener(WORKSPACE_REFRESH_EVENT, handleRefresh);
    return () => window.removeEventListener(WORKSPACE_REFRESH_EVENT, handleRefresh);
  }, [refresh]);

  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

  const toggleExpand = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
  };

  const collapseAll = () => setExpanded(new Set());

  const handleDoubleClick = async (node: FileNode) => {
    if (node.isDir) return;
    try {
      const result = await workspaceApi.getRation(node.path);
      onOpenRation(node.path, result.data);
    } catch (e: any) {
      toast.error(e.message || 'Failed to open ration');
    }
  };

  const handleDelete = async (node: FileNode) => {
    if (!confirm(t('fileExplorer.confirmDelete'))) return;
    try {
      await workspaceApi.deleteItem(node.path);
      toast.success(t('common.delete'));
      void refresh();
    } catch (e: any) {
      toast.error(e.message);
    }
  };

  const handleRenameSubmit = async (node: FileNode) => {
    if (!renameValue.trim()) { setRenamingPath(null); return; }
    const nextNameBase = renameValue.trim();
    const nextName = node.isDir
      ? nextNameBase
      : (nextNameBase.endsWith('.felex.json') ? nextNameBase : `${nextNameBase}.felex.json`);
    const dir = node.path.includes('/') ? node.path.substring(0, node.path.lastIndexOf('/') + 1) : '';
    const newPath = dir + nextName;
    if (newPath === node.path) {
      setRenamingPath(null);
      return;
    }
    try {
      await workspaceApi.rename(node.path, newPath);
      if (!node.isDir && currentProjectPath === node.path) {
        setCurrentProject(newPath, nextName.replace('.felex.json', ''));
      }
      void refresh();
    } catch (e: any) {
      toast.error(e.message);
    }
    setRenamingPath(null);
  };

  const handleNewFolder = async (parentPath: string) => {
    const name = prompt(t('fileExplorer.newFolder'));
    if (!name?.trim()) return;
    const path = parentPath ? `${parentPath}/${name.trim()}` : name.trim();
    try {
      await workspaceApi.createFolder(path);
      setExpanded((prev) => new Set([...prev, parentPath]));
      void refresh();
    } catch (e: any) {
      toast.error(e.message);
    }
  };

  const handleExportCSV = async (node: FileNode) => {
    try {
      const result = await workspaceApi.getRation(node.path);
      await downloadRationCSV(result.data, node.name.replace('.felex.json', '.csv'));
    } catch (e: any) {
      toast.error(e.message);
    }
  };

  const handleExportXLSX = async (node: FileNode) => {
    try {
      const result = await workspaceApi.getRation(node.path);
      await downloadRationXLSX(result.data, node.name.replace('.felex.json', '.xlsx'));
    } catch (e: any) {
      toast.error(e.message);
    }
  };

  const handleMoveToFolder = async (node: FileNode) => {
    const currentFolder = node.path.includes('/') ? node.path.substring(0, node.path.lastIndexOf('/')) : '';
    const requestedFolder = window.prompt(t('fileExplorer.movePrompt'), currentFolder);
    if (requestedFolder === null) {
      return;
    }

    const normalizedFolder = requestedFolder.trim().replace(/^[\\/]+|[\\/]+$/g, '').replace(/\\/g, '/');
    const newPath = normalizedFolder ? `${normalizedFolder}/${node.name}` : node.name;

    if (newPath === node.path) {
      return;
    }

    try {
      await workspaceApi.rename(node.path, newPath);
      if (normalizedFolder) {
        setExpanded((prev) => new Set([...prev, normalizedFolder]));
      }
      if (currentProjectPath === node.path) {
        setCurrentProject(newPath, node.name.replace('.felex.json', ''));
      }
      toast.success(t('fileExplorer.moveSuccess'));
      void refresh();
    } catch (e: any) {
      toast.error(e.message);
    }
  };

  const renderNode = (node: FileNode, depth = 0) => {
    const isExpanded = expanded.has(node.path);
    const isRenaming = renamingPath === node.path;

    return (
      <div key={node.path || 'root'}>
        <div
          className={cn(
            'flex items-center gap-1 px-1 py-0.5 text-xs rounded-[--radius-sm] cursor-pointer',
            'hover:bg-[--bg-hover] group'
          )}
          style={{ paddingLeft: `${depth * 14 + 4}px` }}
          onDoubleClick={() => void handleDoubleClick(node)}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY, node });
          }}
        >
          {node.isDir ? (
            <button onClick={() => toggleExpand(node.path)} className="p-0.5">
              <Icon icon={isExpanded ? ChevronDown : ChevronRight} size={12} className="text-[--text-disabled]" />
            </button>
          ) : (
            <span className="w-5" />
          )}
          <Icon
            icon={node.isDir ? Folder : FileText}
            size={14}
            className={node.isDir ? 'text-[--accent]' : 'text-[--text-secondary]'}
          />
          {isRenaming ? (
            <input
              autoFocus
              className="flex-1 bg-[--bg-surface] border border-[--accent] rounded px-1 text-xs text-[--text-primary] outline-none"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={() => void handleRenameSubmit(node)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void handleRenameSubmit(node);
                if (e.key === 'Escape') setRenamingPath(null);
              }}
            />
          ) : (
            <span className="flex-1 truncate text-[--text-primary]">
              {node.name.replace('.felex.json', '')}
            </span>
          )}
        </div>
        {node.isDir && isExpanded && node.children?.map((child) => renderNode(child, depth + 1))}
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full">
      <div className="px-2 py-1.5 flex items-center gap-1 border-b border-[--border]">
        <span className="text-[10px] font-semibold uppercase text-[--text-disabled] flex-1 tracking-wider">
          {t('fileExplorer.title')}
        </span>
        <button onClick={() => handleNewFolder('')} className="p-1 rounded hover:bg-[--bg-hover]" title={t('fileExplorer.newFolder')}>
          <Icon icon={FolderPlus} size={14} className="text-[--text-disabled]" />
        </button>
        <button onClick={() => onNewRation('')} className="p-1 rounded hover:bg-[--bg-hover]" title={t('fileExplorer.newRation')}>
          <Icon icon={FilePlus} size={14} className="text-[--text-disabled]" />
        </button>
        <button onClick={() => void refresh()} className="p-1 rounded hover:bg-[--bg-hover]" title={t('fileExplorer.refresh')}>
          <Icon icon={RefreshCw} size={14} className="text-[--text-disabled]" />
        </button>
        <button onClick={collapseAll} className="p-1 rounded hover:bg-[--bg-hover]" title={t('fileExplorer.collapseAll')}>
          <Icon icon={ChevronsDownUp} size={14} className="text-[--text-disabled]" />
        </button>
      </div>

      <div className="flex-1 overflow-auto p-1">
        {loading ? (
          <div className="text-xs text-[--text-disabled] p-2">{t('common.loading')}</div>
        ) : !tree?.children?.length ? (
          <div className="text-xs text-[--text-disabled] p-4 text-center">
            {t('fileExplorer.emptyWorkspace')}
          </div>
        ) : (
          tree.children.map((child) => renderNode(child, 0))
        )}
      </div>

      {contextMenu && (
        <div
          className="fixed z-50 bg-[--bg-surface] border border-[--border] rounded-[--radius-md] shadow-lg py-1 min-w-[160px]"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {contextMenu.node.isDir && (
            <>
              <button
                className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
                onClick={() => { onNewRation(contextMenu.node.path); setContextMenu(null); }}
              >
                <Icon icon={FilePlus} size={12} /> {t('fileExplorer.newRation')}
              </button>
              <button
                className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
                onClick={() => { void handleNewFolder(contextMenu.node.path); setContextMenu(null); }}
              >
                <Icon icon={FolderPlus} size={12} /> {t('fileExplorer.newFolder')}
              </button>
              <div className="border-t border-[--border] my-1" />
            </>
          )}
          {!contextMenu.node.isDir && (
            <>
              <button
                className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
                onClick={() => { void handleMoveToFolder(contextMenu.node); setContextMenu(null); }}
              >
                <Icon icon={FolderInput} size={12} /> {t('fileExplorer.moveToFolder')}
              </button>
              <button
                className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
                onClick={() => { void handleExportCSV(contextMenu.node); setContextMenu(null); }}
              >
                <Icon icon={Download} size={12} /> {t('fileExplorer.exportCsv')}
              </button>
              <button
                className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
                onClick={() => { void handleExportXLSX(contextMenu.node); setContextMenu(null); }}
              >
                <Icon icon={Download} size={12} /> {t('fileExplorer.exportXlsx')}
              </button>
              <div className="border-t border-[--border] my-1" />
            </>
          )}
          <button
            className="w-full text-left px-3 py-1.5 text-xs text-[--text-primary] hover:bg-[--bg-hover] flex items-center gap-2"
            onClick={() => {
              setRenamingPath(contextMenu.node.path);
              setRenameValue(contextMenu.node.isDir ? contextMenu.node.name : contextMenu.node.name.replace('.felex.json', ''));
              setContextMenu(null);
            }}
          >
            <Icon icon={Edit2} size={12} /> {t('fileExplorer.rename')}
          </button>
          <button
            className="w-full text-left px-3 py-1.5 text-xs text-red-500 hover:bg-[--bg-hover] flex items-center gap-2"
            onClick={() => { void handleDelete(contextMenu.node); setContextMenu(null); }}
          >
            <Icon icon={Trash2} size={12} /> {t('fileExplorer.delete')}
          </button>
        </div>
      )}
    </div>
  );
}
