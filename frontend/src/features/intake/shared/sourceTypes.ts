import type { TFunction } from 'i18next';
import type { SourceType } from '../../../types/generated';

export const SOURCE_TYPES: readonly SourceType[] = ['webdav', 'local_folder', 's3'];

export function isSourceType(value: unknown): value is SourceType {
  return typeof value === 'string' && (SOURCE_TYPES as readonly string[]).includes(value);
}

/** Display name for a source type; unknown values are shown as-is, missing ones as "Unknown". */
export function sourceTypeLabel(t: TFunction, type?: string | null): string {
  switch (type) {
    case 'webdav':
      return t('intake.sourceType.webdav', 'WebDAV');
    case 'local_folder':
      return t('intake.sourceType.local_folder', 'Local folder');
    case 's3':
      return t('intake.sourceType.s3', 'S3');
    default:
      return type || t('intake.sourceType.unknown', 'Unknown');
  }
}

/** Link into the Ignored section filtered to one connection. */
export function ignoredFilesHref(source: { id: string; name: string; source_type: string }): string {
  const params = new URLSearchParams({
    section: 'ignored',
    sourceType: source.source_type,
    sourceName: source.name,
    sourceId: source.id,
  });
  return `/intake?${params.toString()}`;
}

/** The kind `sourceHue` expects for a connection's type. */
export function kindOfSourceType(type?: string | null): 'webdav' | 's3' | 'local' | undefined {
  if (type === 'webdav' || type === 's3') return type;
  return type === 'local_folder' ? 'local' : undefined;
}
