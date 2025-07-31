import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

const BACKEND_PORT = process.env.BACKEND_PORT || '8000'
const CLIENT_PORT = process.env.CLIENT_PORT || '5173'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['src/test/setup.ts'],
    include: ['**/*.integration.test.{js,jsx,ts,tsx}'],
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
          return
        }
        warn(warning)
      }
    }
  },
})