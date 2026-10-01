import { useId, useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Checkbox, Skeleton } from '../../ui';
import { isShownLit, useAcknowledgeOnLeave } from '../board/litStore';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import { ChangeTag, StatusCell } from './cells';
import { displayName, type LibraryRow } from './data';
import { formatDateTime, formatRelative, ocrState } from './format';
import { groupByMonth, monthLabel } from './months';
import { SourceBadge } from './SourceBadge';
import styles from './LibraryGrid.module.css';

interface LibraryGridProps {
  rows: LibraryRow[];
  selected: ReadonlySet<string>;
  onToggle: (id: string, selected: boolean) => void;
  onOpen: (id: string) => void;
  sourceName: (row: LibraryRow) => string;
  /** Put month headings between the cards (when the order is by date). */
  byMonth: boolean;
  isLoading: boolean;
  emptyState: ReactNode;
  /** Changes whenever the change-tracking store does, so the New markers stay current. */
  litVersion: number;
}

const SKELETON_CARDS = 12;

/** Thumbnail cards, grouped under month headings when the documents are in date order. */
export function LibraryGrid({ rows, selected, onToggle, onOpen, sourceName, byMonth, isLoading, emptyState, litVersion }: LibraryGridProps) {
  const { t, i18n } = useTranslation();
  const rowIds = useMemo(() => rows.map((r) => r.id), [rows]);
  useAcknowledgeOnLeave('document', rowIds);
  const groups = useMemo(
    () => (byMonth ? groupByMonth(rows, (r) => r.created_at) : [{ key: 'all', items: rows }]),
    [rows, byMonth],
  );

  if (isLoading && rows.length === 0) {
    return (
      <div className={styles.grid} role="status" aria-label={t('library.loading', 'Loading documents')}>
        {Array.from({ length: SKELETON_CARDS }, (_, i) => (
          <div key={i} className={styles.skeletonCard} aria-hidden="true">
            <span className={styles.skeletonThumb} />
            <Skeleton lines={2} height={12} />
          </div>
        ))}
      </div>
    );
  }
  if (rows.length === 0) return <div className={styles.empty}>{emptyState}</div>;

  return (
    <div className={styles.groups} aria-busy={isLoading || undefined} data-lit-version={litVersion}>
      {groups.map((g) => (
        <MonthSection
          key={g.key}
          heading={byMonth ? (g.key ? monthLabel(g.key, i18n.language) : t('library.undated', 'Undated')) : null}
          count={g.items.length}
        >
          {g.items.map((row) => (
            <Card
              key={row.id}
              row={row}
              isSelected={selected.has(row.id)}
              onToggle={onToggle}
              onOpen={onOpen}
              sourceName={sourceName(row)}
              lng={i18n.language}
            />
          ))}
        </MonthSection>
      ))}
    </div>
  );
}

function MonthSection({ heading, count, children }: { heading: string | null; count: number; children: ReactNode }) {
  const { t } = useTranslation();
  const id = useId();
  if (heading === null) return <ul className={styles.grid}>{children}</ul>;
  return (
    <section className={styles.month} aria-labelledby={id}>
      <h2 id={id} className={styles.monthHeading}>
        {heading}
        <span className={styles.monthCount}>
          {t('library.monthCount', { count, defaultValue: '{{count}} documents', defaultValue_one: '1 document' })}
        </span>
      </h2>
      <ul className={styles.grid}>{children}</ul>
    </section>
  );
}

interface CardProps {
  row: LibraryRow;
  isSelected: boolean;
  onToggle: (id: string, selected: boolean) => void;
  onOpen: (id: string) => void;
  sourceName: string;
  lng: string;
}

function Card({ row, isSelected, onToggle, onOpen, sourceName, lng }: CardProps) {
  const { t } = useTranslation();
  const name = displayName(row);
  const lit = isShownLit('document', row.id);
  const showStatus = ocrState(row.ocr_status) !== 'completed';
  return (
    <li className={styles.card} data-selected={isSelected || undefined} data-lit={lit || undefined}>
      <button type="button" className={styles.open} onClick={() => onOpen(row.id)}>
        <span className={styles.thumb}>
          <DocumentThumbnail documentId={row.id} mimeType={row.mime_type} size="fill" lazy />
        </span>
        <span className={styles.body}>
          <span className={styles.name}>{name}</span>
          <span className={styles.meta}>
            <SourceBadge row={row} name={sourceName} />
            <time className={styles.date} dateTime={row.created_at} title={formatDateTime(row.created_at, lng)}>
              {formatRelative(row.created_at, lng)}
            </time>
          </span>
        </span>
      </button>
      <span className={styles.flags}>
        <ChangeTag id={row.id} />
        {showStatus ? <StatusCell row={row} /> : null}
      </span>
      <span className={styles.check}>
        <Checkbox
          aria-label={t('library.selectDocument', { name, defaultValue: 'Select {{name}}' })}
          isSelected={isSelected}
          onChange={(next) => onToggle(row.id, next)}
        />
      </span>
    </li>
  );
}
