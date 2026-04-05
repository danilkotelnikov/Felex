import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark' | 'system';

interface ThemeStore {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

export const useTheme = create<ThemeStore>()(
  persist(
    (set) => ({
      theme: 'system',
      setTheme: (theme) => {
        set({ theme });
        applyTheme(theme);
      },
    }),
    {
      name: 'felex-theme',
    }
  )
);

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  const isDark =
    theme === 'dark' ||
    (theme === 'system' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches);
  root.setAttribute('data-theme', isDark ? 'dark' : 'light');
}

// Apply theme on load
if (typeof window !== 'undefined') {
  const stored = localStorage.getItem('felex-theme');
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      applyTheme(parsed.state?.theme || 'system');
    } catch {
      applyTheme('system');
    }
  }
}
