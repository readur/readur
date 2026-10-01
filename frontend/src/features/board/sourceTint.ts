/** Which lane a document came from, for its source colour and name on Home. */
export type LaneKind = 'upload' | 'watch' | 'source';

/** A document's lane: its source (by id), the watch folder, or uploads. */
export function documentLane(doc: { source_id?: string | null; source_type?: string | null }): {
  key: string;
  kind: LaneKind;
} {
  if (doc.source_id) return { key: doc.source_id, kind: 'source' };
  if (doc.source_type === 'watch_folder' || doc.source_type === 'watch') return { key: 'watch', kind: 'watch' };
  return { key: 'upload', kind: 'upload' };
}
