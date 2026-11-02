import { useId, useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, DocumentCard, Skeleton } from '../../ui';
import { shortType } from '../../lib/fileType';
import { isShownLit, useAcknowledgeOnLeave } from '../board/litStore';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import { ChangeTag, LabelsCell, StatusCell } from './cells';
import { displayName, type LibraryRow } from './data';
import { formatBytes, formatDateTime, formatRelative } from './format';
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
          <Card key={i} padding="sm" className={styles.skeletonCard} aria-hidden="true">
            <Skeleton height={160} />
            <Skeleton lines={2} height={12} />
          </Card>
        ))}
      </div>
    );
  }
  if (rows.length === 0) return <Card className={styles.empty}>{emptyState}</Card>;

  return (
    <div className={styles.groups} aria-busy={isLoading || undefined} data-lit-version={litVersion}>
      {groups.map((g) => (
        <MonthSection
          key={g.key}
          heading={byMonth ? (g.key ? monthLabel(g.key, i18n.language) : t('library.undated', 'Undated')) : null}
          count={g.items.length}
        >
          {g.items.map((row) => (
            <GridCard
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

/** "PDF · 412 KB · 2 days ago": type, size and when it arrived, for the card's mono line. */
export function cardMeta(row: Pick<LibraryRow, 'mime_type' | 'file_size' | 'created_at'>, lng: string): string {
  return [shortType(row.mime_type), formatBytes(row.file_size, lng), formatRelative(row.created_at, lng)].join(' · ');
}

function GridCard({ row, isSelected, onToggle, onOpen, sourceName, lng }: CardProps) {
  const { t } = useTranslation();
  const name = displayName(row);
  return (
    <DocumentCard
      title={name}
      thumbnail={<DocumentThumbnail documentId={row.id} mimeType={row.mime_type} size="fill" lazy />}
      meta={
        <time dateTime={row.created_at} title={formatDateTime(row.created_at, lng)}>
          {cardMeta(row, lng)}
        </time>
      }
      status={<StatusCell row={row} />}
      labels={row.labels.length > 0 ? <LabelsCell row={row} /> : undefined}
      source={<SourceBadge row={row} name={sourceName} />}
      flags={<ChangeTag id={row.id} />}
      isChanged={isShownLit('document', row.id)}
      onOpen={() => onOpen(row.id)}
      isSelected={isSelected}
      onSelectionChange={(next) => onToggle(row.id, next)}
      selectLabel={t('library.selectDocument', { name, defaultValue: 'Select {{name}}' })}
    />
  );
}
