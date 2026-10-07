import { useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ButtonLink, DocumentCard, EmptyState, Skeleton, SourceBadge, StatusMark } from '../../ui';
import { Upload } from '../../ui/icons';
import { shortType } from '../../lib/fileType';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import { Label } from '../labels/Label';
import { formatBytes, ocrState } from '../library/format';
import type { SourceArrivals } from './arrivals';
import { fetchRecent, POLL_MS } from './data';
import { formatAge, formatCount } from './format';
import { clearBulkArrivals, isBulkArrival, syncDocuments, useBulkArrivals } from './litFeeders';
import { acknowledge, useAcknowledgeOnLeave, useLitCount, useShownLit } from './litStore';
import { ChangedTag, Region, RegionError } from './Region';
import { documentLane, type LaneKind } from './sourceTint';
import { docName, type BoardDocument } from './types';
import { useResource } from './useResource';
import styles from './Home.module.css';

const DAY_MS = 24 * 3600 * 1000;
/** Two rows of four on a wide screen. */
const SHOWN = 8;
/** Labels shown on a card before "+N". */
const CARD_LABELS = 2;

/** "5 min ago" within a day, otherwise the date. */
function arrivedLabel(iso: string | undefined, locale: string, now: number): string {
  const at = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(at)) return '';
  if (now - at < DAY_MS) return formatAge(iso, locale, now);
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }).format(at);
}

function CardLabels({ doc }: { doc: BoardDocument }) {
  const { t } = useTranslation();
  const labels = doc.labels ?? [];
  if (labels.length === 0) return null;
  const extra = labels.length - CARD_LABELS;
  return (
    <>
      {labels.slice(0, CARD_LABELS).map((l) => (
        <Label key={l.id} label={l} size="small" />
      ))}
      {extra > 0 ? (
        <span className={styles.moreLabels} aria-label={t('library.labelsMore', { count: extra, defaultValue: '{{count}} more' })}>
          +{extra}
        </span>
      ) : null}
    </>
  );
}

function ArrivedCard({ doc, sourceName, now }: { doc: BoardDocument; sourceName: string; now: number }) {
  const { i18n } = useTranslation();
  const lane = documentLane(doc);
  const shown = useShownLit('document', doc.id);
  // Part of a pending bulk import: new with the summary, though not flagged on its own.
  const bulk = useBulkArrivals() > 0 && isBulkArrival(doc.id);
  const lit = shown.lit || bulk;
  const reason = shown.lit ? shown.reason : 'new';
  const progress =
    doc.ocr_progress_total && doc.ocr_progress_total > 0
      ? { current: doc.ocr_progress_current ?? 0, total: doc.ocr_progress_total }
      : undefined;
  const meta = [shortType(doc.mime_type ?? ''), formatBytes(doc.file_size, i18n.language), arrivedLabel(doc.created_at, i18n.language, now)]
    .filter(Boolean)
    .join(' · ');

  return (
    <DocumentCard
      title={docName(doc)}
      href={`/documents/${doc.id}`}
      onOpen={() => acknowledge('document', doc.id)}
      thumbnail={<DocumentThumbnail documentId={doc.id} mimeType={doc.mime_type ?? ''} size="fill" lazy />}
      meta={meta}
      status={<StatusMark state={ocrState(doc.ocr_status)} progress={progress} size="sm" />}
      labels={doc.labels?.length ? <CardLabels doc={doc} /> : undefined}
      source={
        <SourceBadge
          sourceId={lane.kind === 'source' ? lane.key : null}
          kind={lane.kind === 'source' ? doc.source_type : lane.kind}
          type={lane.kind === 'source' ? doc.source_type : lane.kind}
          name={sourceName}
        />
      }
      flags={lit ? <ChangedTag reason={reason} /> : undefined}
      isChanged={lit}
    />
  );
}

export interface JustArrivedProps {
  /** The lanes, to name each document's source. */
  lanes: SourceArrivals[] | undefined;
  now: number;
  className?: string;
}

/** The newest documents as thumbnail cards; items that changed are marked until opened. */
export function JustArrived({ lanes, now, className }: JustArrivedProps) {
  const { t, i18n } = useTranslation();
  const { data, error, reload } = useResource(fetchRecent, POLL_MS);
  useLitCount('document'); // re-render when an item is marked or acknowledged
  const bulk = useBulkArrivals();

  useEffect(() => {
    // On a first visit (nothing seen yet) the last day's arrivals count as new.
    if (data) syncDocuments(data.documents, Date.now() - DAY_MS, data.total);
  }, [data]);

  const docs = useMemo(() => (data?.documents ?? []).slice(0, SHOWN), [data]);
  // Items (and a bulk summary) seen flagged during this visit are acknowledged when the user leaves.
  const ids = useMemo(() => docs.map((d) => d.id), [docs]);
  useAcknowledgeOnLeave('document', ids, bulk > 0 ? clearBulkArrivals : undefined);

  const names = useMemo(() => new Map((lanes ?? []).map((l) => [l.key, l.name])), [lanes]);
  const nameOf = (key: string, kind: LaneKind) =>
    names.get(key) ??
    (kind === 'watch'
      ? t('home.kind.watch', 'Watch folder')
      : kind === 'upload'
        ? t('home.lanes.uploads', 'Uploads')
        : t('home.lanes.source', 'Source'));

  const title = t('home.recent.title', 'Just arrived');
  const upload = (
    <ButtonLink href="/intake?section=upload" variant="primary" icon={<Upload fontSize="inherit" />}>
      {t('home.upload', 'Upload')}
    </ButtonLink>
  );

  return (
    <Region
      surface="plain"
      title={title}
      count={docs.length ? formatCount(docs.length, i18n.language) : undefined}
      className={className}
      headerAction={
        <Link className={styles.link} to="/documents">
          {t('home.recent.viewAll', 'View all')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('board.arrivals.error', 'Recent documents could not be loaded.')} onRetry={reload} />
      ) : !data ? (
        <Skeleton lines={3} label={t('board.loading', 'Loading')} />
      ) : docs.length === 0 ? (
        <EmptyState
          illustration
          title={t('board.arrivals.empty', 'No documents yet')}
          description={t('board.arrivals.emptyHint', 'Documents you add appear here as they arrive.')}
          action={upload}
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
          <ul className={styles.cards} aria-label={title} data-count={docs.length}>
            {docs.map((d) => {
              const lane = documentLane(d);
              return <ArrivedCard key={d.id} doc={d} now={now} sourceName={nameOf(lane.key, lane.kind)} />;
            })}
          </ul>
        </>
      )}
    </Region>
  );
}
