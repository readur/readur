/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_WATCH_FOLDER: string
  // Add more env variables as needed
  readonly VITE_API_URL?: string
  readonly VITE_APP_TITLE?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}