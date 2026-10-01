import { useId, useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { Checkbox, IconButton, Skeleton } from '../../../ui';
import { Visibility } from '../../../ui/icons';
import { useAcknowledgeOnLeave } from '../../board/litStore';
import { DocumentThumbnail } from '../../document/DocumentThumbnail';
import { Label } from '../../labels';
import { ChangeTag, StatusCell } from '../cells';
import { displayName, type LibraryRow } from '../data';
import { formatDateTime, ocrState } from '../format';
import { SnippetLine } from '../Highlight';
import { groupByMonth, monthLabel } from '../months';
import { SourceBadge } from '../SourceBadge';
import { shortType } from '../../../lib/fileType';
import type { SearchSnippet } from '../../../types/generated';
import styles from './Search.module.css';

/** Most snippets shown under one result. */
const MAX_SNIPPETS = 2;

/**
 * The snippets with the most matches, in document order. Snippets that overlap one already
 * chosen (the backend often returns the same passage twice) are skipped.
 */
export function topSnippets(snippets: readonly SearchSnippet[] | undefined, max = MAX_SNIPPETS): SearchSnippet[] {
  if (!snippets || snippets.length === 0) return [];
  const chosen: SearchSnippet[] = [];
  const ranked = [...snippets].sort((a, b) => b.highlight_ranges.length - a.highlight_ranges.length);
  for (const s of ranked) {
    if (chosen.length >= max) break;
    const overlaps = chosen.some(
      (c) => c.text === s.text || (s.start_offset < c.end_offset && c.start_offset < s.end_offset),
    );
    if (!overlaps) chosen.push(s);
  }
  return chosen.sort((a, b) => a.start_offset - b.start_offset);
}

/** Where a result opens: the document page, finding the search words on arrival. */
export function documentHref(id: string, q: string): string {
  const query = q.trim();
  return query ? `/documents/${id}?q=${encodeURIComponent(query)}` : `/documents/${id}`;
}

interface ResultListProps {
  rows: LibraryRow[];
  query: string;
  /** Month headings between results (date order); a flat list in relevance order. */
  byMonth: boolean;
  selected: ReadonlySet<string>;
  onToggle: (id: string, selected: boolean) => void;
  onPreview: (id: string) => void;
  sourceName: (row: LibraryRow) => string;
  isLoading: boolean;
  emptyState: ReactNode;
}

/** Search results: thumbnail, name, source, date and the matching passages, grouped by month. */
export function ResultList({ rows, query, byMonth, selected, onToggle, onPreview, sourceName, isLoading, emptyState }: ResultListProps) {
  const { t, i18n } = useTranslation();
  const rowIds = useMemo(() => rows.map((r) => r.id), [rows]);
  useAcknowledgeOnLeave('document', rowIds);
  const groups = useMemo(() => (byMonth ? groupByMonth(rows, (r) => r.created_at) : null), [rows, byMonth]);

  if (isLoading && rows.length === 0) {
    return (
      <div className={styles.loading} role="status" aria-label={t('library.search.loading', 'Searching…')}>
        {Array.from({ length: 4 }, (_, i) => (
          <div key={i} className={styles.skeletonRow} aria-hidden="true">
            <span className={styles.skeletonThumb} />
            <Skeleton lines={3} height={12} />
          </div>
        ))}
      </div>
    );
  }
  if (rows.length === 0) return <div className={styles.emptyPanel}>{emptyState}</div>;

  const item = (row: LibraryRow) => (
    <Result
      key={row.id}
      row={row}
      query={query}
      isSelected={selected.has(row.id)}
      onToggle={onToggle}
      onPreview={onPreview}
      sourceName={sourceName(row)}
      lng={i18n.language}
    />
  );

  if (!groups) {
    return (
      <section className={styles.group} aria-label={t('library.search.results', 'Results')} aria-busy={isLoading || undefined}>
        <ol className={styles.results}>{rows.map(item)}</ol>
      </section>
    );
  }
  return (
    <div className={styles.groups} aria-busy={isLoading || undefined}>
      {groups.map((g) => (
        <MonthGroup key={g.key} title={g.key ? monthLabel(g.key, i18n.language) : t('library.undated', 'Undated')} count={g.items.length}>
          {g.items.map(item)}
        </MonthGroup>
      ))}
    </div>
  );
}

function MonthGroup({ title, count, children }: { title: string; count: number; children: ReactNode }) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <section className={styles.group} aria-labelledby={id}>
      <h2 id={id} className={styles.monthHeading}>
        {title}
        <span className={styles.monthCount}>
          {t('library.search.monthCount', { count, defaultValue: '{{count}} results', defaultValue_one: '1 result' })}
        </span>
      </h2>
      <ol className={styles.results}>{children}</ol>
    </section>
  );
}

interface ResultProps {
  row: LibraryRow;
  query: string;
  isSelected: boolean;
  onToggle: (id: string, selected: boolean) => void;
  onPreview: (id: string) => void;
  sourceName: string;
  lng: string;
}

function Result({ row, query, isSelected, onToggle, onPreview, sourceName, lng }: ResultProps) {
  const { t } = useTranslation();
  const name = displayName(row);
  const href = documentHref(row.id, query);
  const snippets = topSnippets(row.snippets);
  const notIndexed = ocrState(row.ocr_status) !== 'completed';
  return (
    <li className={styles.result} data-selected={isSelected || undefined}>
      <span className={styles.select}>
        <Checkbox
          aria-label={t('library.selectDocument', { name, defaultValue: 'Select {{name}}' })}
          isSelected={isSelected}
          onChange={(next) => onToggle(row.id, next)}
        />
      </span>
      {/* The name link below is the one in the tab order; the preview is a larger target for the pointer. */}
      <Link to={href} className={styles.thumb} tabIndex={-1} aria-hidden="true">
        <DocumentThumbnail documentId={row.id} mimeType={row.mime_type} size="medium" lazy />
      </Link>
      <div className={styles.main}>
        <h3 className={styles.title}>
          <Link to={href} className={styles.name}>
            {name}
          </Link>
          <ChangeTag id={row.id} />
        </h3>
        <p className={styles.meta}>
          <SourceBadge row={row} name={sourceName} variant="chip" />
          <time dateTime={row.created_at}>{formatDateTime(row.created_at, lng)}</time>
          <span className={styles.type}>{shortType(row.mime_type)}</span>
          {notIndexed ? <StatusCell row={row} /> : null}
          {row.labels.map((l) => (
            <Label key={l.id} label={l} size="small" />
          ))}
        </p>
        {snippets.length > 0 ? (
          <ul className={styles.snippets}>
            {snippets.map((s, i) => (
              <li key={i} className={styles.snippetItem}>
                <SnippetLine snippet={s} />
              </li>
            ))}
          </ul>
        ) : (
          <p className={styles.nameOnly}>{t('library.search.nameMatch', 'Matched on the file name')}</p>
        )}
      </div>
      <span className={styles.preview}>
        <IconButton
          variant="ghost"
          label={t('library.search.preview', { name, defaultValue: 'Quick look at {{name}}' })}
          icon={<Visibility fontSize="small" />}
          onPress={() => onPreview(row.id)}
        />
      </span>
    </li>
  );
}
