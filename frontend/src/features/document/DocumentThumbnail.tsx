import { useEffect, useState } from 'react';
import { documentService } from '../../services/api';
import { Description, Image as ImageIcon, PictureAsPdf, TextSnippet } from '../../ui/icons';
import { fileKind } from './format';
import styles from './DocumentThumbnail.module.css';

export interface DocumentThumbnailProps {
  documentId: string;
  mimeType: string;
  /** 40px, 80px or 120px square. Default `medium`. */
  size?: 'small' | 'medium' | 'large';
  /** Show a file-type icon when there is no thumbnail. Default true. */
  fallbackIcon?: boolean;
}

const ICON_SIZE = { small: 24, medium: 40, large: 56 } as const;

/** Server-rendered thumbnail of a document, or a file-type icon while none exists. */
export function DocumentThumbnail({ documentId, mimeType, size = 'medium', fallbackIcon = true }: DocumentThumbnailProps) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let created: string | null = null;
    setUrl(null);
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

  const box = `${styles.box} ${styles[size]}`;

  if (url) {
    return (
      <span className={box}>
        <img className={styles.image} src={url} alt="" />
      </span>
    );
  }
  if (!fallbackIcon) return null;

  const kind = fileKind(mimeType);
  const Icon = kind === 'pdf' ? PictureAsPdf : kind === 'image' ? ImageIcon : kind === 'text' ? TextSnippet : Description;
  return (
    <span className={`${box} ${styles.icon}`} aria-hidden="true">
      <Icon sx={{ fontSize: ICON_SIZE[size] }} />
    </span>
  );
}

export default DocumentThumbnail;
