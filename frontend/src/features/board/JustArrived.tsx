import { useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { EmptyState, Skeleton } from '../../ui';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import type { SourceArrivals } from './arrivals';
import { fetchRecent, POLL_MS } from './data';
import { formatAge, formatCount } from './format';
import { LaneDot } from './LaneDot';
import { clearBulkArrivals, syncDocuments, useBulkArrivals } from './litFeeders';
import { acknowledge, useAcknowledgeOnLeave, useLitCount, useShownLit } from './litStore';
import { ChangedTag, Region, RegionError } from './Region';
import { documentLane, sourceHue } from './sourceTint';
import { docName, type BoardDocument } from './types';
import { useResource } from './useResource';
import styles from './Home.module.css';

const DAY_MS = 24 * 3600 * 1000;

/** "5 min ago" within a day, otherwise the date. */
function arrivedLabel(iso: string | undefined, locale: string, now: number): string {
  const at = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(at)) return '';
  if (now - at < DAY_MS) return formatAge(iso, locale, now);
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }).format(at);
}

interface CardProps {
  doc: BoardDocument;
  sourceName: string;
  hue: number;
  now: number;
}

function Card({ doc, sourceName, hue, now }: CardProps) {
  const { i18n } = useTranslation();
  const { lit, reason } = useShownLit('document', doc.id);
  const name = docName(doc);
  return (
    <li className={styles.card} data-changed={lit || undefined}>
      <Link className={styles.cardLink} to={`/documents/${doc.id}`} onClick={() => acknowledge('document', doc.id)}>
        <span className={styles.cardThumb}>
          <DocumentThumbnail documentId={doc.id} mimeType={doc.mime_type ?? ''} size="fill" lazy />
          {lit ? <ChangedTag reason={reason} className={styles.cardTag} /> : null}
        </span>
        <span className={styles.cardName} title={name}>
          {name}
        </span>
      </Link>
      <span className={styles.cardMeta}>
        <LaneDot hue={hue} />
        <span className={styles.cardSource}>{sourceName}</span>
        <span className={styles.cardDate}>{arrivedLabel(doc.created_at, i18n.language, now)}</span>
      </span>
    </li>
  );
}

export interface JustArrivedProps {
  /** The lanes, to name each document's source. */
  lanes: SourceArrivals[] | undefined;
  now: number;
}

/** The newest documents as page thumbnails; items that changed are marked until opened. */
export function JustArrived({ lanes, now }: JustArrivedProps) {
  const { t, i18n } = useTranslation();
  const { data, error, reload } = useResource(fetchRecent, POLL_MS);
  useLitCount('document'); // re-render when an item is marked or acknowledged
  const bulk = useBulkArrivals();

  useEffect(() => {
    if (data) syncDocuments(data.documents, undefined, data.total);
  }, [data]);

  // Items (and a bulk summary) seen flagged during this visit are acknowledged when the user leaves.
  const ids = useMemo(() => (data?.documents ?? []).map((d) => d.id), [data]);
  useAcknowledgeOnLeave('document', ids, bulk > 0 ? clearBulkArrivals : undefined);

  const names = useMemo(() => new Map((lanes ?? []).map((l) => [l.key, l.name])), [lanes]);
  const nameOf = (key: string, kind: string) =>
    names.get(key) ??
    (kind === 'watch'
      ? t('home.kind.watch', 'Watch folder')
      : kind === 'upload'
        ? t('home.lanes.uploads', 'Uploads')
        : t('home.lanes.source', 'Source'));

  const title = t('home.recent.title', 'Just arrived');
  const add = (
    <Link className={styles.primaryLink} to="/intake?section=upload">
      {t('board.addDocuments', 'Add documents')}
    </Link>
  );

  return (
    <Region
      title={title}
      headerAction={
        <Link className={styles.link} to="/documents">
          {t('board.arrivals.library', 'Open library')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('board.arrivals.error', 'Recent documents could not be loaded.')} onRetry={reload} />
      ) : !data ? (
        <div className={styles.panelBody}>
          <Skeleton lines={3} label={t('board.loading', 'Loading')} />
        </div>
      ) : data.documents.length === 0 ? (
        <EmptyState
          title={t('board.arrivals.empty', 'No documents yet')}
          description={t('board.arrivals.emptyHint', 'Documents you add appear here as they arrive.')}
          action={add}
        />
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
          <ul className={styles.cards} aria-label={title}>
            {data.documents.map((d) => {
              const lane = documentLane(d);
              return (
                <Card
                  key={d.id}
                  doc={d}
                  now={now}
                  sourceName={nameOf(lane.key, lane.kind)}
                  hue={sourceHue(lane.key === 'upload' || lane.key === 'watch' ? null : lane.key, lane.kind).index}
                />
              );
            })}
          </ul>
        </>
      )}
    </Region>
  );
}
