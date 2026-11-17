import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Support environment variables for development
const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
const CLIENT_PORT = process.env.CLIENT_PORT || '5173';
// Allow overriding the proxy target for Docker development
const PROXY_TARGET = process.env.VITE_API_PROXY_TARGET || `http://localhost:${BACKEND_PORT}`;

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
  },
  server: {
    port: parseInt(CLIENT_PORT),
    proxy: {
      '/api': {
        target: PROXY_TARGET,
        // Keep the browser's Host header: the server accepts a WebSocket handshake only when
        // its Origin matches Host (src/routes/sources/progress_ws.rs), and the browser's Origin
        // is this dev server.
        changeOrigin: false,
        // Sync progress streams over a WebSocket under /api (see services/api/syncProgress.ts).
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    rollupOptions: {
      onwarn(warning, warn) {
        // Suppress module-level directive warnings ("use client") from dependencies
        if (warning.code === 'MODULE_LEVEL_DIRECTIVE') {
          return;
        }
        warn(warning);
      },
    },
  },
});
