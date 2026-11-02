/**
 * Helpers for displaying user-supplied document content.
 *
 * Blob URLs created from API responses share the application's origin, so
 * content that a browser can execute (HTML, SVG, XML, ...) must never be
 * rendered top-level or in an unsandboxed frame.
 */

const SAFE_INLINE_MIME_TYPES = new Set([
  'application/pdf',
  'image/png',
  'image/jpeg',
  'image/jpg',
  'image/gif',
  'image/webp',
  'image/bmp',
  'image/tiff',
  'text/plain',
])

/** True when a document of this MIME type can be opened directly in a browser tab. */
export const isSafeInlineMime = (mimeType: string | undefined | null): boolean =>
  !!mimeType && SAFE_INLINE_MIME_TYPES.has(mimeType.split(';')[0].trim().toLowerCase())

/**
 * `sandbox` attribute value for an iframe previewing user content.
 * PDFs are left unsandboxed because the built-in PDF viewers do not work in
 * sandboxed frames; everything else gets the most restrictive sandbox.
 */
export const previewSandbox = (mimeType: string | undefined | null): string | undefined =>
  mimeType?.split(';')[0].trim().toLowerCase() === 'application/pdf' ? undefined : ''
