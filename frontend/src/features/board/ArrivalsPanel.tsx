import { useCallback, useEffect, useMemo } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BoardTable, EmptyState, StatusMark, type BoardColumn } from '../../ui';
import { fetchArrivals, POLL_MS } from './data';
import { fileTypeLabel, formatAge, formatBytes, formatCount } from './format';
import { clearBulkArrivals, syncDocuments, useBulkArrivals } from './litFeeders';
import { acknowledge, isShownLit, useAcknowledgeOnLeave, useLitCount, useShownLit } from './litStore';
import { ChangedTag, Region, RegionError } from './Region';
import { docName, documentState, type BoardDocument } from './types';
import { useResource } from './useResource';
import styles from './Board.module.css';

function NameCell({ doc }: { doc: BoardDocument }) {
  const { lit, reason } = useShownLit('document', doc.id);
  return (
    <span className={styles.name}>
      {lit ? <ChangedTag reason={reason} /> : null}
      <span className={styles.nameText}>{docName(doc)}</span>
    </span>
  );
}

/** The ten newest documents, refreshed every 15 seconds; rows that changed stay marked until opened. */
export function ArrivalsPanel() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { data, loading, error, reload } = useResource(fetchArrivals, POLL_MS);
  useLitCount('document'); // re-render when a row is marked or acknowledged

  const bulk = useBulkArrivals();

  useEffect(() => {
    if (data) syncDocuments(data.documents, undefined, data.total);
  }, [data]);

  // Rows (and a bulk summary) seen flagged during this visit are acknowledged when the user leaves.
  const bulkSeen = bulk > 0;
  const rowIds = useMemo(() => (data?.documents ?? []).map((d) => d.id), [data]);
  useAcknowledgeOnLeave('document', rowIds, bulkSeen ? clearBulkArrivals : undefined);

  const open = useCallback(
    (id: string) => {
      acknowledge('document', id);
      navigate(`/documents/${id}`);
    },
    [navigate],
  );

  const columns = useMemo<BoardColumn<BoardDocument>[]>(
    () => [
      { id: 'name', label: t('board.col.name', 'Name'), isRowHeader: true, render: (d) => <NameCell doc={d} /> },
      {
        id: 'type',
        hideOnNarrow: true,
        label: t('board.col.type', 'Type'),
        width: 72,
        render: (d) => <span className={styles.mono}>{fileTypeLabel(docName(d), d.mime_type)}</span>,
      },
      {
        id: 'status',
        label: t('board.col.status', 'Status'),
        width: 130,
        render: (d) => {
          const total = d.ocr_progress_total ?? 0;
          const progress =
            documentState(d) === 'processing' && total > 0
              ? { current: d.ocr_progress_current ?? 0, total }
              : undefined;
          return <StatusMark state={documentState(d)} progress={progress} size="sm" />;
        },
      },
      { id: 'size', hideOnNarrow: true, label: t('board.col.size', 'Size'), width: 96, align: 'end', mono: true, render: (d) => formatBytes(d.file_size, i18n.language) },
      { id: 'added', hideOnNarrow: true, label: t('board.col.added', 'Added'), width: 96, align: 'end', mono: true, render: (d) => formatAge(d.created_at, i18n.language) },
    ],
    [t, i18n.language],
  );

  const title = t('board.arrivals.title', 'Recently added');
  const add = (
    <Link className={styles.linkButton} to="/intake?section=upload">
      {t('board.addDocuments', 'Add documents')}
    </Link>
  );

  return (
    <Region
      className={styles.arrivals}
      title={title}
      headerAction={
        <Link className={styles.link} to="/documents">
          {t('board.arrivals.library', 'Open library')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('board.arrivals.error', 'Recent documents could not be loaded.')} onRetry={reload} />
      ) : data && data.total === 0 && data.documents.length === 0 ? (
        <div className={styles.empty}>
          <EmptyState
            title={t('board.arrivals.empty', 'No documents yet')}
            description={t('board.arrivals.emptyHint', 'Documents you add appear here as they arrive.')}
            action={add}
          />
        </div>
      ) : (
        <>
          {error ? <RegionError message={t('board.arrivals.error', 'Recent documents could not be loaded.')} onRetry={reload} /> : null}
          {bulk > 0 ? (
            <p className={styles.bulk} role="status">
              <ChangedTag reason="new" />
              <span>
                <span className={styles.bulkCount}>{formatCount(bulk, i18n.language)}</span>{' '}
                {bulk === 1 ? t('board.arrivals.bulkOne', 'new document') : t('board.arrivals.bulkMany', 'new documents')}
              </span>
              <Link className={styles.link} to="/documents?sort=created_at&order=desc" onClick={clearBulkArrivals}>
                {t('board.arrivals.bulkOpen', 'Open in Library, newest first')}
              </Link>
            </p>
          ) : null}
          <BoardTable
            aria-label={title}
            density="compact"
            columns={columns}
            rows={data?.documents ?? []}
            getRowId={(d) => d.id}
            isLoading={loading}
            isRowLit={(d) => isShownLit('document', d.id)}
            onRowAction={open}
          />
        </>
      )}
    </Region>
  );
}
