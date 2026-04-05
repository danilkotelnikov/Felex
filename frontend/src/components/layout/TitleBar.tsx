import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import {
  Sun,
  Moon,
  Monitor,
  Settings,
  Check,
  ChevronRight,
  FilePlus,
  FolderOpen,
  Save,
  FileOutput,
  LogOut,
  Undo,
  Redo,
  Info,
} from 'lucide-react';
import { Icon } from '../ui/Icon';
import { Button } from '../ui/Button';
import { useTheme, type Theme } from '@/lib/theme';
import { cn } from '@/lib/utils';
import { calculateLocalNutrients, useRationStore } from '@/stores/rationStore';
import { exportCsv, exportExcel, exportPdf } from '@/lib/export';
import { useResolvedNormReference } from '@/lib/resolved-norms';

const themeIcons: Record<Theme, typeof Sun> = {
  light: Sun,
  dark: Moon,
  system: Monitor,
};

export function TitleBar() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const { clearRation, setActiveView, localItems, animalProperties, customNorms, activeNormPresetId } = useRationStore();
  const [showAbout, setShowAbout] = useState(false);
  const { norms: currentNorms } = useResolvedNormReference(
    animalProperties,
    activeNormPresetId,
    customNorms,
  );

  const hasExportData = localItems.length > 0;

  const handleNewRation = () => {
    if (confirm(`${t('menu.newRation')}?`)) {
      clearRation();
    }
  };

  const getExportContext = () => {
    const nutrients = localItems.length > 0 ? calculateLocalNutrients(localItems) : null;
    return { nutrients, norms: currentNorms };
  };

  const handleExportPdf = async () => {
    if (!hasExportData) return;
    const { nutrients, norms } = getExportContext();
    await exportPdf(localItems, nutrients, norms);
  };

  const handleExportCsv = async () => {
    if (!hasExportData) return;
    const { nutrients } = getExportContext();
    await exportCsv(localItems, nutrients);
  };

  const handleExportExcel = async () => {
    if (!hasExportData) return;
    const { nutrients, norms } = getExportContext();
    await exportExcel(localItems, nutrients, norms);
  };

  return (
    <header
      className="h-9 flex items-center justify-between px-3 border-b"
      style={{
        background: 'var(--bg-elevated)',
        borderColor: 'var(--border)',
      }}
    >
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <img src="/icon.png" alt="Felex" className="w-5 h-5 rounded object-cover" />
          <span className="text-sm font-semibold text-[--text-primary]">Felex</span>
        </div>

        <nav className="flex items-center gap-0.5">
          <DropdownMenu.Root>
            <DropdownMenu.Trigger asChild>
              <Button variant="ghost" size="sm" className="px-2 h-6">
                {t('menu.file')}
              </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className={menuContentClass} sideOffset={4}>
                <DropdownMenu.Item className={menuItemClass} onClick={handleNewRation}>
                  <Icon icon={FilePlus} size={14} />
                  {t('menu.newRation')}
                </DropdownMenu.Item>
                <DropdownMenu.Item className={menuItemClass}>
                  <Icon icon={FolderOpen} size={14} />
                  {t('menu.open')}
                </DropdownMenu.Item>
                <DropdownMenu.Item className={menuItemClass}>
                  <Icon icon={Save} size={14} />
                  {t('menu.saveAs')}
                </DropdownMenu.Item>
                <DropdownMenu.Separator className={separatorClass} />
                <DropdownMenu.Sub>
                  <DropdownMenu.SubTrigger className={menuItemClass}>
                    <Icon icon={FileOutput} size={14} />
                    {t('menu.export')}
                    <Icon icon={ChevronRight} size={14} className="ml-auto" />
                  </DropdownMenu.SubTrigger>
                  <DropdownMenu.Portal>
                    <DropdownMenu.SubContent className={menuContentClass} sideOffset={4}>
                      <DropdownMenu.Item className={menuItemClass} onClick={handleExportPdf} disabled={!hasExportData}>
                        {t('menu.exportPdf')}
                      </DropdownMenu.Item>
                      <DropdownMenu.Item className={menuItemClass} onClick={handleExportExcel} disabled={!hasExportData}>
                        {t('menu.exportExcel')}
                      </DropdownMenu.Item>
                      <DropdownMenu.Item className={menuItemClass} onClick={handleExportCsv} disabled={!hasExportData}>
                        {t('report.exportCsv')}
                      </DropdownMenu.Item>
                    </DropdownMenu.SubContent>
                  </DropdownMenu.Portal>
                </DropdownMenu.Sub>
                <DropdownMenu.Separator className={separatorClass} />
                <DropdownMenu.Item className={menuItemClass}>
                  <Icon icon={LogOut} size={14} />
                  {t('menu.exit')}
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger asChild>
              <Button variant="ghost" size="sm" className="px-2 h-6">
                {t('menu.edit')}
              </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className={menuContentClass} sideOffset={4}>
                <DropdownMenu.Item className={menuItemClass} disabled>
                  <Icon icon={Undo} size={14} />
                  {t('menu.undo')}
                  <span className="ml-auto text-[10px] text-[--text-disabled]">Ctrl+Z</span>
                </DropdownMenu.Item>
                <DropdownMenu.Item className={menuItemClass} disabled>
                  <Icon icon={Redo} size={14} />
                  {t('menu.redo')}
                  <span className="ml-auto text-[10px] text-[--text-disabled]">Ctrl+Y</span>
                </DropdownMenu.Item>
                <DropdownMenu.Separator className={separatorClass} />
                <DropdownMenu.Item className={menuItemClass} onClick={() => setActiveView('settings')}>
                  <Icon icon={Settings} size={14} />
                  {t('menu.preferences')}
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger asChild>
              <Button variant="ghost" size="sm" className="px-2 h-6">
                {t('menu.view')}
              </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className={menuContentClass} sideOffset={4}>
                <DropdownMenu.Sub>
                  <DropdownMenu.SubTrigger className={menuItemClass}>
                    <Icon icon={themeIcons[theme]} size={14} />
                    {t('menu.theme')}
                    <Icon icon={ChevronRight} size={14} className="ml-auto" />
                  </DropdownMenu.SubTrigger>
                  <DropdownMenu.Portal>
                    <DropdownMenu.SubContent className={menuContentClass} sideOffset={4}>
                      <DropdownMenu.RadioGroup value={theme} onValueChange={(value) => setTheme(value as Theme)}>
                        <DropdownMenu.RadioItem className={menuItemClass} value="light">
                          <DropdownMenu.ItemIndicator className="mr-2">
                            <Icon icon={Check} size={12} />
                          </DropdownMenu.ItemIndicator>
                          <Icon icon={Sun} size={14} />
                          {t('menu.lightTheme')}
                        </DropdownMenu.RadioItem>
                        <DropdownMenu.RadioItem className={menuItemClass} value="dark">
                          <DropdownMenu.ItemIndicator className="mr-2">
                            <Icon icon={Check} size={12} />
                          </DropdownMenu.ItemIndicator>
                          <Icon icon={Moon} size={14} />
                          {t('menu.darkTheme')}
                        </DropdownMenu.RadioItem>
                        <DropdownMenu.RadioItem className={menuItemClass} value="system">
                          <DropdownMenu.ItemIndicator className="mr-2">
                            <Icon icon={Check} size={12} />
                          </DropdownMenu.ItemIndicator>
                          <Icon icon={Monitor} size={14} />
                          {t('menu.systemTheme')}
                        </DropdownMenu.RadioItem>
                      </DropdownMenu.RadioGroup>
                    </DropdownMenu.SubContent>
                  </DropdownMenu.Portal>
                </DropdownMenu.Sub>
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>

          <DropdownMenu.Root>
            <DropdownMenu.Trigger asChild>
              <Button variant="ghost" size="sm" className="px-2 h-6">
                {t('menu.help')}
              </Button>
            </DropdownMenu.Trigger>
            <DropdownMenu.Portal>
              <DropdownMenu.Content className={menuContentClass} sideOffset={4}>
                <DropdownMenu.Item className={menuItemClass} onClick={() => setShowAbout(true)}>
                  <Icon icon={Info} size={14} />
                  {t('menu.about')}
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Portal>
          </DropdownMenu.Root>
        </nav>
      </div>

      <div className="flex items-center gap-1">
        <Button variant="ghost" size="sm" title={t('nav.settings')} onClick={() => setActiveView('settings')}>
          <Icon icon={Settings} size={16} />
        </Button>
      </div>

      {showAbout && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setShowAbout(false)}>
          <div className="bg-[--bg-surface] rounded-[--radius-lg] p-6 max-w-sm shadow-xl" onClick={(event) => event.stopPropagation()}>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-12 h-12 rounded-lg bg-[--accent] flex items-center justify-center">
                <span className="text-2xl font-bold text-white">F</span>
              </div>
              <div>
                <h2 className="text-lg font-semibold text-[--text-primary]">Felex</h2>
                <p className="text-xs text-[--text-secondary]">v1.0</p>
              </div>
            </div>
            <p className="text-xs text-[--text-secondary] mb-4">Feed. Calculate. Optimize.</p>
            <p className="text-[10px] text-[--text-disabled] mb-4">
              © 2026 Felex - система составления рационов для КРС, свиней и птицы.
            </p>
            <Button size="sm" onClick={() => setShowAbout(false)} className="w-full">
              {t('common.close')}
            </Button>
          </div>
        </div>
      )}
    </header>
  );
}

const menuContentClass = cn(
  'min-w-[180px] bg-[--bg-surface] border border-[--border] rounded-[--radius-md] shadow-lg p-1 z-50',
  'animate-in fade-in-0 zoom-in-95 data-[side=bottom]:slide-in-from-top-2'
);

const menuItemClass = cn(
  'flex items-center gap-2 px-2 py-1.5 text-xs text-[--text-primary] rounded-[--radius-sm]',
  'cursor-pointer outline-none',
  'hover:bg-[--bg-hover] focus:bg-[--bg-hover]',
  'data-[disabled]:opacity-50 data-[disabled]:pointer-events-none'
);

const separatorClass = 'h-px bg-[--border] my-1';

