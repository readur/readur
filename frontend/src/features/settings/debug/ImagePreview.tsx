import { useEffect, useRef, useState } from 'react';
import styles from './Debug.module.css';

/** Raster image types that are previewed; SVG and anything else is never decoded. */
const PREVIEWABLE = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/bmp']);
const MAX_WIDTH = 640;
const MAX_HEIGHT = 400;

function isPreviewable(file: File | null | undefined): file is File {
  return !!file && PREVIEWABLE.has(file.type) && typeof createImageBitmap === 'function';
}

/**
 * Draws a selected raster image onto a canvas. The file is decoded to pixels
 * and painted; no URL derived from it is ever placed in the DOM.
 */
export function ImagePreview({ file, caption }: { file: File | null | undefined; caption: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [failedFile, setFailedFile] = useState<File | null>(null);
  const show = isPreviewable(file) && failedFile !== file;

  useEffect(() => {
    if (!isPreviewable(file)) return;
    let cancelled = false;
    createImageBitmap(file).then(
      (bitmap) => {
        const canvas = canvasRef.current;
        const ctx = canvas && !cancelled ? canvas.getContext('2d') : null;
        if (canvas && ctx) {
          const scale = Math.min(1, MAX_WIDTH / bitmap.width, MAX_HEIGHT / bitmap.height);
          canvas.width = Math.max(1, Math.round(bitmap.width * scale));
          canvas.height = Math.max(1, Math.round(bitmap.height * scale));
          ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
        } else if (!cancelled) {
          setFailedFile(file);
        }
        bitmap.close();
      },
      () => {
        if (!cancelled) setFailedFile(file);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [file]);

  if (!show) return null;
  return (
    <figure className={styles.figure}>
      <figcaption className={styles.panelTitle}>{caption}</figcaption>
      <canvas ref={canvasRef} role="img" aria-label={file.name} className={styles.previewCanvas} />
    </figure>
  );
}
