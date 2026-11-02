import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { documentService } from '../../services/api';
import { shortType } from '../../lib/fileType';
import { fileKind } from './format';
import { cachedThumbnail, hasNoThumbnail, loadThumbnail, requestThumbnail } from './thumbnailLoader';
import styles from './DocumentThumbnail.module.css';

export interface DocumentThumbnailProps {
  documentId: string;
  mimeType: string;
  /**
   * `row` is a 40×28 stub for table rows; `small`, `medium` and `large` are 40, 80 and 120px
   * squares; `fill` takes its container's width at 4:3. Default `medium`.
   */
  size?: 'row' | 'small' | 'medium' | 'large' | 'fill';
  /** Show the type-code stub when there is no thumbnail. Default true. */
  fallbackIcon?: boolean;
  /** Shown under the type code once it is known there is no preview (for example "No preview yet"). */
  emptyText?: ReactNode;
  /**
   * For long lists: load only once the thumbnail scrolls into view, through a shared queue
   * (a few requests at a time) and cache, so remounting or scrolling back does not refetch.
   */
  lazy?: boolean;
}

/**
 * The server only renders real previews of images and PDFs; for everything else (and for PDFs it
 * cannot read) it returns a flat coloured square, so those are not fetched at all.
 */
export function hasPreview(mimeType: string): boolean {
  const kind = fileKind(mimeType);
  return kind === 'image' || kind === 'pdf';
}

/**
 * Server-rendered thumbnail of a document, letterboxed in a fixed box so it is never squashed or
 * cropped; or, while there is none, a hairline stub with the mono type code (PDF, DOCX, PNG).
 */
export function DocumentThumbnail({
  documentId,
  mimeType,
  size = 'medium',
  fallbackIcon = true,
  emptyText,
  lazy = false,
}: DocumentThumbnailProps) {
  const previewable = hasPreview(mimeType);
  const eager = useEagerThumbnail(lazy || !previewable ? null : documentId);
  const holder = useRef<HTMLSpanElement>(null);
  const lazyState = useLazyThumbnail(lazy && previewable ? documentId : null, holder);
  const state: ThumbState = !previewable ? NONE : lazy ? lazyState : eager;

  const box = `${styles.box} ${styles[size]}`;

  if (state.url) {
    return (
      <span className={`${box} ${styles.frame}`} ref={holder}>
        <img className={styles.image} src={state.url} alt="" />
      </span>
    );
  }
  if (!fallbackIcon) return lazy ? <span ref={holder} className={box} aria-hidden="true" /> : null;

  const showEmpty = state.done && emptyText;
  return (
    <span ref={holder} className={`${box} ${styles.frame} ${styles.stub}`} data-state={state.done ? 'none' : 'loading'}>
      <span className={styles.code} aria-hidden="true">
        {shortType(mimeType)}
      </span>
      {showEmpty ? <span className={styles.emptyText}>{emptyText}</span> : null}
    </span>
  );
}

interface ThumbState {
  url: string | null;
  /** True once it is known whether there is a thumbnail. */
  done: boolean;
}

const NONE: ThumbState = { url: null, done: true };
const PENDING: ThumbState = { url: null, done: false };

/** Fetch as soon as mounted; the object URL is owned (and revoked) by this component. */
function useEagerThumbnail(documentId: string | null): ThumbState {
  const [state, setState] = useState<ThumbState>(PENDING);
  useEffect(() => {
    setState(PENDING);
    if (!documentId) return undefined;
    let cancelled = false;
    let created: string | null = null;
    loadThumbnail(documentId, documentService.getThumbnail)
      .then((url) => {
        if (cancelled) {
          if (url) URL.revokeObjectURL(url);
          return;
        }
        created = url;
        setState({ url, done: true });
      })
      .catch(() => {
        // No thumbnail yet: the stub covers it.
        if (!cancelled) setState(NONE);
      });
    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [documentId]);
  return state;
}

/** What the shared cache already knows about a document's thumbnail. */
function initial(id: string | null): ThumbState {
  if (!id) return PENDING;
  const hit = cachedThumbnail(id);
  if (hit) return { url: hit, done: true };
  return hasNoThumbnail(id) ? NONE : PENDING;
}

/** Fetch once visible, through the shared queue and cache (which owns the object URL). */
function useLazyThumbnail(documentId: string | null, holder: RefObject<HTMLSpanElement | null>): ThumbState {
  const [state, setState] = useState<ThumbState>(() => initial(documentId));
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    setState(initial(documentId));
    setVisible(false);
  }, [documentId]);

  useEffect(() => {
    const el = holder.current;
    if (!documentId || visible || !el) return undefined;
    if (typeof IntersectionObserver === 'undefined') {
      setVisible(true);
      return undefined;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: '200px 0px' },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [documentId, visible, holder]);

  useEffect(() => {
    if (!documentId || !visible || state.done) return undefined;
    return requestThumbnail(documentId, documentService.getThumbnail, (next) => setState({ url: next, done: true }));
  }, [documentId, visible, state.done]);

  return state;
}

export default DocumentThumbnail;
