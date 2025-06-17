import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Support environment variables for development
const BACKEND_PORT = process.env.BACKEND_PORT || '8000'
const CLIENT_PORT = process.env.CLIENT_PORT || '5173'

export default defineConfig({
  plugins: [react()],
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
        // Suppress "use client" directive warnings from MUI
        if (warning.code === 'MODULE_LEVEL_DIRECTIVE') {
          return
        }
        warn(warning)
      }
    }
  },
})