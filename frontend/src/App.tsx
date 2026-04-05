import { useEffect } from 'react';
import { Toaster } from 'react-hot-toast';
import { AppLayout } from './components/layout/AppLayout';
import { BackendGate } from './components/layout/BackendGate';
import { useTheme } from './lib/theme';
import { useRationStore, calculateLocalNutrients } from './stores/rationStore';

function App() {
  const { theme } = useTheme();
  const { localItems, setNutrients, nutrients } = useRationStore();

  // Apply theme on mount
  useEffect(() => {
    const root = document.documentElement;
    const isDark =
      theme === 'dark' ||
      (theme === 'system' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.setAttribute('data-theme', isDark ? 'dark' : 'light');
  }, [theme]);

  // Initialize nutrients from sample data on mount
  useEffect(() => {
    if (localItems.length > 0 && !nutrients) {
      setNutrients(calculateLocalNutrients(localItems));
    }
  }, [localItems, nutrients, setNutrients]);

  return (
    <BackendGate>
      <AppLayout />
      <Toaster
        position="bottom-right"
        toastOptions={{
          style: {
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            border: '1px solid var(--border)',
          },
        }}
      />
    </BackendGate>
  );
}

export default App;
