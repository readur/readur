import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Button, EmptyState, Pagination, type Selection } from '../../ui';
import { Add } from '../../ui/icons';
import { acknowledge, useLitCount } from '../board/litStore';
import { PageHeader } from '../shell';
import { BulkActions } from './BulkActions';
import type { LibraryRow } from './data';
import { DetailPanel } from './DetailPanel';
import { FilterStrip } from './FilterStrip';
import { formatCount } from './format';
import { LibraryTable } from './LibraryTable';
import { useCompactRows } from './useCompactRows';
import { useFacets, useRows } from './useLibraryData';
import { PAGE_SIZES, hasFilters, isSearch, useLibraryQuery } from './urlState';
import styles from './Library.module.css';

const EMPTY_SELECTION: Selection = new Set();

/** /documents: every document on one sortable, filterable board, with search and a detail panel. */
export function Library() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { query, update, clearSort, clearFilters } = useLibraryQuery();
  const facets = useFacets();
  const { rows, total, status, reload, patchRow } = useRows(query, facets.mimeTypes, facets.mimeReady);
  const [compact, setCompact] = useCompactRows();
  const litVersion = useLitCount('document');
  const searching = isSearch(query);

  // Selection belongs to what is on screen: any change of page, filter or query clears it.
  const [selection, setSelection] = useState<Selection>(EMPTY_SELECTION);
  const viewKey = JSON.stringify(query);
  useEffect(() => setSelection(EMPTY_SELECTION), [viewKey]);

  const [openId, setOpenId] = useState<string | null>(null);
  const [panelOpen, setPanelOpen] = useState(false);
  // Keep the last row shown while the panel animates closed or the row leaves the page.
  const lastRow = useRef<LibraryRow | null>(null);
  const openRow = rows.find((r) => r.id === openId) ?? null;
  if (openRow) lastRow.current = openRow;

  const open = useCallback((id: string) => {
    acknowledge('document', id);
    setOpenId(id);
    setPanelOpen(true);
  }, []);

  const navigateRows = (direction: 'previous' | 'next') => {
    const index = rows.findIndex((r) => r.id === openId);
    if (index === -1) return;
    const next = rows[index + (direction === 'next' ? 1 : -1)];
    if (next) open(next.id);
  };

  const sourceById = useMemo(() => new Map(facets.sources.map((s) => [s.id, s.name])), [facets.sources]);
  const sourceName = useCallback(
    (row: LibraryRow) => {
      if (!row.source_id) return t('library.source.upload', 'Upload');
      return sourceById.get(row.source_id) ?? row.source_type ?? t('library.source.unknown', 'Connection');
    },
    [sourceById, t],
  );

  const selectedRows = selection === 'all' ? rows : rows.filter((r) => selection.has(r.id));
  const filtered = hasFilters(query) || searching;

  const emptyState =
    status === 'tooMany' ? (
      <EmptyState
        title={t('library.empty.tooMany', 'Too many matches — refine your search')}
        description={t('library.empty.tooManyHint', 'Add a word or a filter to narrow the results.')}
      />
    ) : status === 'error' ? (
      <EmptyState
        title={t('library.empty.error', 'Documents could not be loaded')}
        action={<Button onPress={reload}>{t('library.retry', 'Try again')}</Button>}
      />
    ) : filtered ? (
      <EmptyState
        title={t('library.empty.noMatches', 'No matches')}
        description={t('library.empty.noMatchesHint', 'Nothing fits the current search and filters.')}
        action={
          <Button onPress={() => update({ q: '', types: [], labels: [], status: null, sources: [], from: null, to: null })}>
            {t('library.empty.clearFilters', 'Clear filters')}
          </Button>
        }
      />
    ) : (
      <EmptyState
        title={t('library.empty.none', 'No documents yet')}
        description={t('library.empty.noneHint', 'Upload files or connect a folder to start your library.')}
        action={
          <Button variant="primary" icon={<Add fontSize="small" />} onPress={() => navigate('/intake?section=upload')}>
            {t('library.add', 'Add documents')}
          </Button>
        }
      />
    );

  const sort = searching && !query.sortExplicit
    ? undefined
    : { column: query.sort, direction: query.order === 'asc' ? ('ascending' as const) : ('descending' as const) };

  return (
    <div className={styles.page}>
      <PageHeader
        title={t('library.title', 'Library')}
        meta={
          <span aria-live="polite">
            {status === 'loading' && total === 0
              ? t('library.counting', 'Counting…')
              : t('library.count', { count: total, formatted: formatCount(total, i18n.language), defaultValue: '{{formatted}} documents' })}
          </span>
        }
        actions={
          <Button variant="primary" icon={<Add fontSize="small" />} onPress={() => navigate('/intake?section=upload')}>
            {t('library.add', 'Add documents')}
          </Button>
        }
      />
      <FilterStrip
        query={query}
        update={update}
        clearSort={clearSort}
        clearFilters={clearFilters}
        labels={facets.labels}
        sources={facets.sources}
        compact={compact}
        onCompactChange={setCompact}
      />
      <LibraryTable
        rows={status === 'ready' || status === 'loading' ? rows : []}
        sort={sort}
        onSortChange={(field, order) => update({ sort: field, order })}
        selectedKeys={selection}
        onSelectionChange={setSelection}
        onOpen={open}
        sourceName={sourceName}
        showSnippets={searching}
        compact={compact}
        isLoading={status === 'loading'}
        emptyState={emptyState}
        litVersion={litVersion}
      />
      {total > 0 ? (
        <Pagination
          page={query.page}
          pageSize={query.size}
          total={total}
          pageSizeOptions={[...PAGE_SIZES]}
          onChange={(page, size) => update(size !== query.size ? { page: 1, size } : { page })}
        />
      ) : null}
      <DetailPanel
        row={openRow ?? lastRow.current}
        isOpen={panelOpen}
        onOpenChange={setPanelOpen}
        onNavigate={navigateRows}
        query={searching ? query.q : ''}
        sourceName={sourceName}
        availableLabels={facets.labels}
        onLabelCreated={facets.addLabel}
        onRowChange={patchRow}
        onDeleted={() => {
          setPanelOpen(false);
          reload();
        }}
      />
      <BulkActions
        selected={selectedRows}
        onClear={() => setSelection(EMPTY_SELECTION)}
        availableLabels={facets.labels}
        onLabelCreated={facets.addLabel}
        onChanged={reload}
      />
    </div>
  );
}

export default Library;
