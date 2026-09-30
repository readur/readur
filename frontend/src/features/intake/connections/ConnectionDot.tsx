import { SourceDot } from '../../../ui/SourceDot';
import { kindOfSourceType } from '../shared/sourceTypes';

/** The connection's colour, the same one the sidebar and Home use for it. */
export function ConnectionDot({ id, type }: { id: string; type?: string | null }) {
  return <SourceDot sourceId={id} kind={kindOfSourceType(type)} />;
}
