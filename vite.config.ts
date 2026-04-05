import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  root: 'frontend',
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './frontend/src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:7432',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    // The full DB-derived feed detail artifact is intentionally lazy-loaded as a dedicated chunk.
    chunkSizeWarningLimit: 7500,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('feed-catalog.generated.json')) {
            return 'feed-library';
          }
          if (id.includes('feed-details.generated.json')) {
            return 'feed-details.generated';
          }
          if (!id.includes('node_modules')) {
            return undefined;
          }
          if (id.includes('xlsx') || id.includes('papaparse')) {
            return 'export';
          }
          if (id.includes('recharts')) {
            return 'charts';
          }
          if (id.includes('react-markdown') || id.includes('remark-gfm')) {
            return 'markdown';
          }
          if (id.includes('@dnd-kit')) {
            return 'dragdrop';
          }
          if (id.includes('@radix-ui') || id.includes('lucide-react')) {
            return 'ui';
          }
          if (id.includes('@tanstack/react-table') || id.includes('@tanstack/react-query')) {
            return 'data';
          }
          return undefined;
        },
      },
    },
  },
});
