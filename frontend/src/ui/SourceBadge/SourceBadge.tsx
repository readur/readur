import type { ReactNode } from 'react';
import { Cloud, Folder, FolderSearch, Server, Upload } from 'lucide-react';
import { sourceHue, type SourceKind } from '../../lib/sourceColor';
import { cx } from '../shared/FieldParts';
import styles from './SourceBadge.module.css';

export type SourceType = 'webdav' | 's3' | 'local' | 'local_folder' | 'upload' | 'direct_upload' | 'watch' | 'watch_folder';

const ICON: Record<string, typeof Cloud> = {
  webdav: Cloud,
  s3: Server,
  local: Folder,
  local_folder: Folder,
  upload: Upload,
  direct_upload: Upload,
  watch: FolderSearch,
  watch_folder: FolderSearch,
};
const TYPE_LABEL: Record<string, string> = {
  webdav: 'WebDAV',
  s3: 'S3',
  local: 'Local',
  local_folder: 'Local',
  upload: 'Upload',
  direct_upload: 'Upload',
  watch: 'Watch',
  watch_folder: 'Watch',
};

export interface SourceBadgeProps {
  sourceId: string | null | undefined;
  /** Colour family for `sourceHue` ('upload', 'watch' or a source type). */
  kind?: SourceKind | string | null;
  name: ReactNode;
  /** Picks the icon (and the optional caption). Defaults to `kind`. */
  type?: SourceType | string | null;
  /** `tile` is the icon tile plus the name; `chip` sits both on an outlined pill. */
  variant?: 'tile' | 'chip';
  /** Show the type ("WebDAV", "S3"…) in the data face after the name. */
  showType?: boolean;
  className?: string;
}

/** Where something came from: a tile in the source's hue with its type icon, plus the name (never colour alone). */
export function SourceBadge({ sourceId, kind, name, type, variant = 'tile', showType = false, className }: SourceBadgeProps) {
  const { index } = sourceHue(sourceId, kind);
  const key = type ?? kind ?? 'upload';
  const Icon = ICON[key] ?? Folder;
  return (
    <span className={cx(styles.badge, variant === 'chip' && styles.chip, className)} data-slot={index}>
      <span className={styles.tile} data-tile="" aria-hidden="true">
        <Icon width={13} height={13} strokeWidth={2} />
      </span>
      <span className={styles.name}>{name}</span>
      {showType && TYPE_LABEL[key] ? <span className={styles.type}>{TYPE_LABEL[key]}</span> : null}
    </span>
  );
}
