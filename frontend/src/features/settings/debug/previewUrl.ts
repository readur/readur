/** Raster image types that are safe to preview; SVG and anything else is not shown inline. */
const PREVIEWABLE = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/bmp']);

/**
 * Object URL for previewing a selected image, or null when the file is not a
 * previewable raster image. The result is only ever a `blob:` URL created by
 * the browser for this file, never a string built from file metadata.
 */
export function previewObjectUrl(file: File | null | undefined): string | null {
  if (!file || !PREVIEWABLE.has(file.type) || typeof URL.createObjectURL !== 'function') return null;
  const url = URL.createObjectURL(file);
  return url.startsWith('blob:') ? url : null;
}
