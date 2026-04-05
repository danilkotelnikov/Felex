import { invoke } from '@tauri-apps/api/core';

export const isTauriApp = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function openExternalUrl(url: string): Promise<void> {
  if (isTauriApp) {
    await invoke('open_external_url', { url });
    return;
  }

  window.open(url, '_blank', 'noopener,noreferrer');
}
