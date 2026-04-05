import type { RationProject, FileNode } from '@/types/ration-project';
import { isTauriApp } from './desktop';

const BASE = isTauriApp ? 'http://localhost:7432/api/v1' : '/api/v1';
const HEALTH_URL = isTauriApp ? 'http://localhost:7432/health' : '/health';

const REQUEST_TIMEOUT_MS = 8000;
const MAX_RETRIES = 3;
const RETRY_DELAYS = [500, 1500, 3000];

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const method = options?.method ?? 'GET';
  const isIdempotent = method === 'GET';
  const maxAttempts = isIdempotent ? MAX_RETRIES : 1;

  let lastError: Error | undefined;

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    if (attempt > 0) {
      const delay = RETRY_DELAYS[attempt - 1] ?? 3000;
      await new Promise((resolve) => setTimeout(resolve, delay));
    }

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

    let res: Response;
    try {
      res = await fetch(url, {
        headers: { 'Content-Type': 'application/json' },
        ...options,
        signal: controller.signal,
      });
    } catch (error) {
      clearTimeout(timeout);

      if (attempt < maxAttempts - 1) {
        lastError =
          error instanceof Error ? error : new Error(String(error));
        continue;
      }

      const message = isTauriApp
        ? 'Cannot reach the embedded Felex API on localhost:7432. The backend may still be starting — please wait a moment and retry.'
        : 'Cannot reach the Felex API. Start the backend with `npm run dev:full` or make sure `/api/v1` is available.';
      console.error('Workspace API request failed', error);
      throw new Error(message);
    } finally {
      clearTimeout(timeout);
    }

    if (!res.ok) {
      const err = await res.text();
      throw new Error(err || res.statusText);
    }

    if (res.status === 204) {
      return {} as T;
    }

    const raw = await res.text();
    if (!raw.trim()) {
      return {} as T;
    }

    try {
      return JSON.parse(raw) as T;
    } catch {
      return raw as T;
    }
  }

  throw lastError ?? new Error('Request failed after retries');
}

export const workspaceApi = {
  getTree: () =>
    request<{ data: FileNode[] }>(`${BASE}/workspace/tree`),

  createFolder: (path: string) =>
    request(`${BASE}/workspace/folder`, {
      method: 'POST',
      body: JSON.stringify({ path }),
    }),

  createRation: (path: string, project: RationProject) =>
    request(`${BASE}/workspace/ration`, {
      method: 'POST',
      body: JSON.stringify({ path, project }),
    }),

  getRation: (path: string) =>
    request<{ data: RationProject }>(`${BASE}/workspace/ration?path=${encodeURIComponent(path)}`),

  updateRation: (path: string, project: RationProject) =>
    request(`${BASE}/workspace/ration`, {
      method: 'PUT',
      body: JSON.stringify({ path, project }),
    }),

  deleteItem: (path: string) =>
    request(`${BASE}/workspace/ration?path=${encodeURIComponent(path)}`, {
      method: 'DELETE',
    }),

  rename: (oldPath: string, newPath: string) =>
    request(`${BASE}/workspace/rename`, {
      method: 'POST',
      body: JSON.stringify({ old_path: oldPath, new_path: newPath }),
    }),

  getConfig: () =>
    request<{ data: { workspace_root: string } }>(`${BASE}/workspace/config`),

  updateConfig: (workspaceRoot: string) =>
    request(`${BASE}/workspace/config`, {
      method: 'PUT',
      body: JSON.stringify({ workspace_root: workspaceRoot }),
    }),
};

/**
 * Poll the backend /health endpoint until it responds 200,
 * or until `maxWaitMs` elapses.
 */
export async function waitForBackend(
  maxWaitMs = 30000,
  pollIntervalMs = 500,
): Promise<boolean> {
  const deadline = Date.now() + maxWaitMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(HEALTH_URL, {
        signal: AbortSignal.timeout(2000),
      });
      if (res.ok) return true;
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, pollIntervalMs));
  }
  return false;
}
