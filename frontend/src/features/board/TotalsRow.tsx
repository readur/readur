import { useTranslation } from 'react-i18next';
import { Pass, PassCell, Skeleton } from '../../ui';
import { fetchLabelCount, fetchTotals } from './data';
import { formatBytes, formatCount } from './format';
import { Region, RegionError } from './Region';
import { useResource } from './useResource';
import styles from './Board.module.css';

/** Library totals: documents, storage, share indexed, labels. */
export function TotalsRow() {
  const { t, i18n } = useTranslation();
  const totals = useResource(fetchTotals);
  const labels = useResource(fetchLabelCount);
  const title = t('board.totals.title', 'Library');
  const d = totals.data;
  const indexed = d && d.documents > 0 ? Math.round((d.withOcr / d.documents) * 100) : 0;
  const labelCount = labels.data;

  return (
    <Region className={styles.totals} title={title}>
      {totals.error && !d ? (
        <RegionError message={t('board.totals.error', 'Library totals could not be loaded.')} onRetry={totals.reload} />
      ) : !d ? (
        <div className={styles.body}>
          <Skeleton lines={1} label={t('board.loading', 'Loading')} />
        </div>
      ) : (
        <Pass aria-label={title}>
          <PassCell label={t('board.totals.documents', 'Documents')} mono>{formatCount(d.documents, i18n.language)}</PassCell>
          <PassCell label={t('board.totals.storage', 'Storage')} mono>{formatBytes(d.storageBytes, i18n.language)}</PassCell>
          <PassCell label={t('board.totals.indexed', 'Indexed')} mono>{`${indexed}%`}</PassCell>
          <PassCell label={t('board.totals.labels', 'Labels')} mono>
            {labelCount === null || labelCount === undefined ? '—' : formatCount(labelCount, i18n.language)}
          </PassCell>
        </Pass>
      )}
    </Region>
  );
}
