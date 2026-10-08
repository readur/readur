import { SourceTile } from '../../../ui/SourceBadge';
import { kindOfSourceType } from '../shared/sourceTypes';

/** The connection's icon tile, in the same hue the sidebar and Home use for it. */
export function ConnectionDot({ id, type }: { id: string; type?: string | null }) {
  return <SourceTile sourceId={id} kind={kindOfSourceType(type)} type={type} />;
}
