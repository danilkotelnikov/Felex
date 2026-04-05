import { useState, useEffect, type ReactNode } from 'react';
import { waitForBackend } from '@/lib/workspace-api';

interface BackendGateProps {
  children: ReactNode;
}

export function BackendGate({ children }: BackendGateProps) {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    waitForBackend(30000, 500).then((ok) => {
      if (cancelled) return;
      if (ok) setReady(true);
      else setError(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-[--bg-base]">
        <div className="text-center space-y-3">
          <div className="text-sm text-[--status-error]">
            Failed to connect to backend
          </div>
          <button
            onClick={() => {
              setError(false);
              setReady(false);
              waitForBackend(15000, 500).then((ok) => {
                if (ok) setReady(true);
                else setError(true);
              });
            }}
            className="rounded-[--radius-md] bg-[--accent] px-4 py-1.5 text-xs text-white"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!ready) {
    return (
      <div className="flex h-screen items-center justify-center bg-[--bg-base]">
        <div className="text-center space-y-2">
          <div className="h-6 w-6 mx-auto animate-spin rounded-full border-2 border-[--border] border-t-[--accent]" />
          <div className="text-xs text-[--text-secondary]">
            Starting Felex...
          </div>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
