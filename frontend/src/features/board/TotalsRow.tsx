import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../ui';
import { fetchLabelCount, fetchTotals } from './data';
import { Figure, Figures } from './Figures';
import { formatBytes, formatCount } from './format';
import { Region, RegionError } from './Region';
import { useResource } from './useResource';
import styles from './Board.module.css';

/** Library totals (documents, storage, share indexed, labels) as a segment of the pass strip. */
export function TotalsRow() {
  const { t, i18n } = useTranslation();
  const totals = useResource(fetchTotals);
  const labels = useResource(fetchLabelCount);
  const title = t('board.totals.title', 'Library');
  const d = totals.data;
  const indexed = d && d.documents > 0 ? Math.round((d.withOcr / d.documents) * 100) : 0;
  const labelCount = labels.data;

  return (
    <Region variant="segment" title={title}>
      {totals.error && !d ? (
        <div className={styles.segmentBody}>
          <RegionError message={t('board.totals.error', 'Library totals could not be loaded.')} onRetry={totals.reload} />
        </div>
      ) : !d ? (
        <div className={styles.segmentBody}>
          <Skeleton lines={1} label={t('board.loading', 'Loading')} />
        </div>
      ) : (
        <Figures aria-label={title}>
          <Figure label={t('board.totals.documents', 'Documents')}>{formatCount(d.documents, i18n.language)}</Figure>
          <Figure label={t('board.totals.storage', 'Storage')}>{formatBytes(d.storageBytes, i18n.language)}</Figure>
          <Figure label={t('board.totals.indexed', 'Indexed')}>{`${indexed}%`}</Figure>
          <Figure label={t('board.totals.labels', 'Labels')}>
            {labelCount === null || labelCount === undefined ? '—' : formatCount(labelCount, i18n.language)}
          </Figure>
        </Figures>
      )}
    </Region>
  );
}
