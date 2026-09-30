import { useEffect, useRef, useState, type RefObject } from 'react';
import { documentService } from '../../services/api';
import { Description, Image as ImageIcon, PictureAsPdf, TextSnippet } from '../../ui/icons';
import { fileKind } from './format';
import { cachedThumbnail, requestThumbnail } from './thumbnailLoader';
import styles from './DocumentThumbnail.module.css';

export interface DocumentThumbnailProps {
  documentId: string;
  mimeType: string;
  /** 40px, 80px or 120px square. Default `medium`. */
  size?: 'small' | 'medium' | 'large';
  /** Show a file-type icon when there is no thumbnail. Default true. */
  fallbackIcon?: boolean;
  /**
   * For long lists: load only once the thumbnail scrolls into view, through a shared queue
   * (a few requests at a time) and cache, so remounting or scrolling back does not refetch.
   */
  lazy?: boolean;
}

const ICON_SIZE = { small: 24, medium: 40, large: 56 } as const;

/** Server-rendered thumbnail of a document, or a file-type icon while none exists. */
export function DocumentThumbnail({
  documentId,
  mimeType,
  size = 'medium',
  fallbackIcon = true,
  lazy = false,
}: DocumentThumbnailProps) {
  const eager = useEagerThumbnail(lazy ? null : documentId);
  const holder = useRef<HTMLSpanElement>(null);
  const lazyUrl = useLazyThumbnail(lazy ? documentId : null, holder);
  const url = lazy ? lazyUrl : eager;

  const box = `${styles.box} ${styles[size]}`;

  if (url) {
    return (
      <span className={box} ref={holder}>
        <img className={styles.image} src={url} alt="" />
      </span>
    );
  }
  if (!fallbackIcon) return lazy ? <span ref={holder} className={box} aria-hidden="true" /> : null;

  const kind = fileKind(mimeType);
  const Icon = kind === 'pdf' ? PictureAsPdf : kind === 'image' ? ImageIcon : kind === 'text' ? TextSnippet : Description;
  return (
    <span ref={holder} className={`${box} ${styles.icon}`} aria-hidden="true">
      <Icon sx={{ fontSize: ICON_SIZE[size] }} />
    </span>
  );
}

/** Fetch as soon as mounted; the object URL is owned (and revoked) by this component. */
function useEagerThumbnail(documentId: string | null): string | null {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    setUrl(null);
    if (!documentId) return undefined;
    let cancelled = false;
    let created: string | null = null;
    documentService
      .getThumbnail(documentId)
      .then((res) => {
        if (cancelled) return;
        created = URL.createObjectURL(new Blob([res.data]));
        setUrl(created);
      })
      .catch(() => {
        // No thumbnail yet: the icon fallback covers it.
      });
    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [documentId]);
  return url;
}

/** Fetch once visible, through the shared queue and cache (which owns the object URL). */
function useLazyThumbnail(documentId: string | null, holder: RefObject<HTMLSpanElement | null>): string | null {
  const [url, setUrl] = useState<string | null>(() => (documentId ? cachedThumbnail(documentId) : null));
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    setUrl(documentId ? cachedThumbnail(documentId) : null);
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
    if (!documentId || !visible || url) return undefined;
    return requestThumbnail(documentId, documentService.getThumbnail, (next) => setUrl(next));
  }, [documentId, visible, url]);

  return url;
}

export default DocumentThumbnail;
