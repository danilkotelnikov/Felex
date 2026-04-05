export const WORKSPACE_REFRESH_EVENT = 'felex:workspace-refresh';

export function requestWorkspaceRefresh() {
  if (typeof window === 'undefined') {
    return;
  }

  window.dispatchEvent(new CustomEvent(WORKSPACE_REFRESH_EVENT));
}
