import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

const BACKEND_PORT = process.env.BACKEND_PORT || '8000';
const CLIENT_PORT = process.env.CLIENT_PORT || '5173';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['src/test/setup.ts'],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/cypress/**',
      '**/.{idea,git,cache,output,temp}/**',
      '**/e2e/**',
      '**/*.e2e.test.{js,jsx,ts,tsx}',
      '**/*.integration.test.{js,jsx,ts,tsx}',
    ],
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        '**/*.test.{ts,tsx}',
        'src/test/**',
        '**/*.d.ts',
        'src/**/__tests__/**',
        'src/main.tsx',
      ],
      reporter: ['text-summary', 'json-summary', 'html'],
      // Floor(measured) - 1 per metric. Thresholds only go up: raise them when coverage improves.
      // Re-based once for vitest 5, whose v8 remapping counts statements/functions on a
      // different basis (same code: 23089 -> 9247 statements), not because coverage fell.
      thresholds: {
        statements: 86,
        branches: 80,
        functions: 84,
        lines: 88,
      },
    },
  },
  server: {
    port: parseInt(CLIENT_PORT),
    proxy: {
      '/api': {
        target: `http://localhost:${BACKEND_PORT}`,
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    rollupOptions: {
      onwarn(warning, warn) {
        if (warning.code === 'MODULE_LEVEL_DIRECTIVE') {
          return;
        }
        warn(warning);
      },
    },
  },
});
