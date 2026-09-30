/**
 * The server watch folder settings shown to users. The API does not expose them yet, so these
 * mirror the server defaults (VITE_WATCH_FOLDER overrides the path), as the previous page did.
 */
export const SYSTEM_WATCH = {
  folder: (import.meta.env.VITE_WATCH_FOLDER as string | undefined) || './watch',
  intervalSeconds: 30,
  maxFileAgeHours: 24,
  allowedTypes: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt', 'doc', 'docx'],
  strategy: 'hybrid',
} as const;
