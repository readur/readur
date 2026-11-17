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
        // Send the target's own Host, so name-based and HTTPS upstreams work.
        changeOrigin: true,
        // Sync progress streams over a WebSocket under /api (see services/api/syncProgress.ts).
        ws: true,
        configure: (proxy) => {
          // The server accepts a WebSocket handshake only from an Origin it recognises
          // (src/routes/sources/progress_ws.rs). The browser's Origin is this dev server, which
          // no longer matches Host once changeOrigin rewrites it, so tell the server which host
          // the browser addressed. It believes X-Forwarded-Host only from TRUSTED_PROXIES
          // (loopback and private networks by default), and still compares it with the
          // untouched Origin, so a foreign page stays rejected.
          proxy.on('proxyReqWs', (proxyReq, req) => {
            if (!req.headers.host) return;
            const encrypted = 'encrypted' in req.socket && Boolean(req.socket.encrypted);
            proxyReq.setHeader('X-Forwarded-Host', req.headers.host);
            proxyReq.setHeader('X-Forwarded-Proto', encrypted ? 'https' : 'http');
          });
        },
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
