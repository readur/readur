import { useTranslation } from 'react-i18next';
import { Button } from '../../ui';
import { clearBulkArrivals, useBulkArrivals } from './litFeeders';
import { acknowledgeAll, useShownLitCount, type LitKind } from './litStore';

export interface MarkAllSeenProps {
  /** The kinds of lit rows this board shows. */
  kinds: readonly LitKind[];
}

/**
 * Ghost "Mark all seen" action for a board that shows lit rows. Renders nothing while none of
 * the given kinds has a lit row (and no bulk-arrival summary is pending).
 */
export function MarkAllSeen({ kinds }: MarkAllSeenProps) {
  const { t } = useTranslation();
  const counts: Record<LitKind, number> = {
    document: useShownLitCount('document'),
    source: useShownLitCount('source'),
    attention: useShownLitCount('attention'),
  };
  const bulk = useBulkArrivals();
  const withDocuments = kinds.includes('document');
  const lit = kinds.reduce((sum, kind) => sum + counts[kind], 0) + (withDocuments ? bulk : 0);
  if (lit === 0) return null;

  return (
    <Button
      variant="ghost"
      onPress={() => {
        kinds.forEach((kind) => acknowledgeAll(kind));
        if (withDocuments) clearBulkArrivals();
      }}
    >
      {t('board.markAllSeen', 'Mark all seen')}
    </Button>
  );
}
