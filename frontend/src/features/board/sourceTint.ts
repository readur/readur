/**
 * Source colour for a lane or a document. Uploads and the watch folder have fixed slots; a
 * source's slot comes from its id.
 */
export type SourceKind = 'watch' | 'upload' | 'webdav' | 's3' | 'local';

export interface SourceHue {
  varName: string;
  softVarName: string;
  index: number;
}

function hash(id: string): number {
  let h = 2166136261;
  for (let i = 0; i < id.length; i += 1) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function sourceHue(sourceId: string | null, kind?: SourceKind): SourceHue {
  const index = kind === 'upload' ? 1 : kind === 'watch' ? 2 : sourceId ? (hash(sourceId) % 6) + 3 : 1;
  return { varName: `--src-${index}`, softVarName: `--src-${index}-soft`, index };
}

/** The server's lane kind (`local_folder`, …) as a colour kind. */
export function tintKind(kind: string | null | undefined): SourceKind | undefined {
  switch (kind) {
    case 'upload':
    case 'watch':
    case 'webdav':
    case 's3':
      return kind;
    case 'local_folder':
      return 'local';
    default:
      return undefined;
  }
}

/** Which lane a document came from: its source, the watch folder, or uploads. */
export function documentLane(doc: { source_id?: string | null; source_type?: string | null }): {
  key: string;
  kind: SourceKind;
} {
  if (doc.source_id) return { key: doc.source_id, kind: tintKind(doc.source_type) ?? 'local' };
  if (doc.source_type === 'watch_folder' || doc.source_type === 'watch') return { key: 'watch', kind: 'watch' };
  return { key: 'upload', kind: 'upload' };
}
