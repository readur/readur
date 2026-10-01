import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { Button, EmptyState, Pagination, Switch, type Selection } from '../../ui';
import { Add, GridView, TableChart } from '../../ui/icons';
import { acknowledge, useLitCount } from '../board/litStore';
import { MarkAllSeen } from '../board/MarkAllSeen';
import { swatchStyle } from '../labels/labelData';
import { PageHeader } from '../shell';
import { BulkActions } from './BulkActions';
import type { LibraryRow } from './data';
import { DetailPanel } from './DetailPanel';
import { FilterStrip, SortSelect } from './FilterStrip';
import { formatCount } from './format';
import { LibraryGrid } from './LibraryGrid';
import { LibraryTable } from './LibraryTable';
import { SearchBox } from './SearchBox';
import { Segmented } from './Segmented';
import { useCompactRows } from './useCompactRows';
import { useFacets, useRows, useSourceName } from './useLibraryData';
import { useLibraryView, type LibraryView } from './useLibraryView';
import { NO_FILTERS, PAGE_SIZES, hasFilters, isSearch, parseQuery, toParams, useLibraryQuery, type LibraryQuery } from './urlState';
import styles from './Library.module.css';

const EMPTY_SELECTION: Selection = new Set();
const SEEN_KINDS = ['document'] as const;

/**
 * /documents. A search (`?q=`) belongs to the Search page, so it moves there with every filter
 * kept; the typing carries on in the Search page's field.
 */
export function Library() {
  const [params] = useSearchParams();
  const query = parseQuery(params);
  if (isSearch(query)) {
    return <Navigate to={`/search?${toParams(query).toString()}`} replace state={{ focusSearch: true }} />;
  }
  return <LibraryBoard />;
}

/** Every document as thumbnails by month or as the dense table, with filters and a detail panel. */
function LibraryBoard() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { query, update, clearFilters } = useLibraryQuery();
  const facets = useFacets();
  const { rows, total, status, reload, patchRow } = useRows(query, facets.mimeTypes, facets.mimeReady);
  const [compact, setCompact] = useCompactRows();
  const [view, setView] = useLibraryView();
  const litVersion = useLitCount('document');
  const sourceName = useSourceName(facets.sources);

  // Selection belongs to what is on screen: any change of page, filter or layout clears it.
  const [selection, setSelection] = useState<Selection>(EMPTY_SELECTION);
  const viewKey = JSON.stringify(query) + view;
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

  // A deleted document must not stay open (or reopen from the last-row fallback).
  const closeIfDeleted = useCallback(
    (ids: string[]) => {
      if (openId && ids.includes(openId)) {
        setPanelOpen(false);
        setOpenId(null);
        lastRow.current = null;
      }
    },
    [openId],
  );
  // The same when the open document disappears from a fresh page of results.
  useEffect(() => {
    if (panelOpen && status === 'ready' && openId && !rows.some((r) => r.id === openId)) {
      closeIfDeleted([openId]);
    }
  }, [panelOpen, status, openId, rows, closeIfDeleted]);

  const navigateRows = (direction: 'previous' | 'next') => {
    const index = rows.findIndex((r) => r.id === openId);
    if (index === -1) return;
    const next = rows[index + (direction === 'next' ? 1 : -1)];
    if (next) open(next.id);
  };

  const selectedIds = useMemo(
    () => (selection === 'all' ? new Set(rows.map((r) => r.id)) : (selection as Set<string>)),
    [selection, rows],
  );
  const selectedRows = rows.filter((r) => selectedIds.has(r.id));
  const toggle = useCallback((id: string, on: boolean) => {
    setSelection((prev) => {
      const next = new Set(prev === 'all' ? [] : (prev as Set<string>));
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  // A single collection (label) filter makes this the collection's page.
  const collection = query.labels.length === 1 ? facets.labels.find((l) => l.id === query.labels[0]) ?? null : null;
  const shownRows = status === 'ready' || status === 'loading' ? rows : [];
  const emptyState = (
    <LibraryEmpty
      status={status}
      collection={collection && onlyCollection(query) ? collection.name : null}
      onSearch={() => navigate('/search')}
      filtered={hasFilters(query)}
      onRetry={reload}
      onClear={() => update(NO_FILTERS)}
      onAdd={() => navigate('/intake?section=upload')}
    />
  );

  return (
    <div className={styles.page}>
      <PageHeader
        title={
          collection ? (
            <span className={styles.collectionTitle}>
              <span className={styles.collectionSwatch} style={swatchStyle(collection.color) as CSSProperties} aria-hidden="true" />
              {collection.name}
            </span>
          ) : (
            t('library.title', 'Library')
          )
        }
        meta={collection?.description ? <span>{collection.description}</span> : undefined}
        figure={
          <span aria-live="polite">
            {status === 'loading' && total === 0
              ? t('library.counting', 'Counting…')
              : t('library.count', { count: total, formatted: formatCount(total, i18n.language), defaultValue: '{{formatted}} documents', defaultValue_one: '{{formatted}} document' })}
          </span>
        }
        actions={
          <>
            <MarkAllSeen kinds={SEEN_KINDS} />
            <Button variant="primary" icon={<Add fontSize="small" />} onPress={() => navigate('/intake?section=upload')}>
              {t('library.add', 'Add documents')}
            </Button>
          </>
        }
      />
      <FilterStrip
        query={query}
        update={update}
        clearFilters={clearFilters}
        labels={facets.labels}
        sources={facets.sources}
        search={
          <SearchBox
            value=""
            onChange={(q) => update({ q })}
            commitOn="submit"
            placeholder={t('library.search.everything', 'Search every document…')}
          />
        }
        controls={<ViewControls view={view} onViewChange={setView} compact={compact} onCompactChange={setCompact} query={query} update={update} />}
      />
      {view === 'grid' ? (
        <LibraryGrid
          rows={shownRows}
          selected={selectedIds}
          onToggle={toggle}
          onOpen={open}
          sourceName={sourceName}
          byMonth={query.sort === 'created_at'}
          isLoading={status === 'loading'}
          emptyState={emptyState}
          litVersion={litVersion}
        />
      ) : (
        <LibraryTable
          rows={shownRows}
          sort={{ column: query.sort, direction: query.order === 'asc' ? 'ascending' : 'descending' }}
          onSortChange={(field, order) => update({ sort: field, order })}
          selectedKeys={selection}
          onSelectionChange={setSelection}
          onOpen={open}
          sourceName={sourceName}
          showSnippets={false}
          compact={compact}
          isLoading={status === 'loading'}
          emptyState={emptyState}
          litVersion={litVersion}
        />
      )}
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
        query=""
        sourceName={sourceName}
        availableLabels={facets.labels}
        onLabelCreated={facets.addLabel}
        onRowChange={patchRow}
        onDeleted={(id) => {
          closeIfDeleted([id]);
          reload();
        }}
      />
      <BulkActions
        selected={selectedRows}
        onClear={() => setSelection(EMPTY_SELECTION)}
        availableLabels={facets.labels}
        onLabelCreated={facets.addLabel}
        onChanged={reload}
        onDeleted={closeIfDeleted}
      />
    </div>
  );
}

interface ViewControlsProps {
  view: LibraryView;
  onViewChange: (view: LibraryView) => void;
  compact: boolean;
  onCompactChange: (compact: boolean) => void;
  query: LibraryQuery;
  update: ReturnType<typeof useLibraryQuery>['update'];
}

function ViewControls({ view, onViewChange, compact, onCompactChange, query, update }: ViewControlsProps) {
  const { t } = useTranslation();
  return (
    <>
      {/* The table sorts from its column heads; the grid (and a phone) needs the menu. */}
      <SortSelect query={query} update={update} className={view === 'table' ? styles.mobileSort : styles.sort} />
      {view === 'table' ? (
        <Switch isSelected={compact} onChange={onCompactChange} className={styles.density} label={t('library.compact', 'Compact rows')} />
      ) : null}
      <Segmented
        label={t('library.view.label', 'Layout')}
        iconOnly
        value={view}
        onChange={onViewChange}
        items={[
          { id: 'grid', label: t('library.view.grid', 'Grid'), icon: <GridView fontSize="small" /> },
          { id: 'table', label: t('library.view.table', 'Table'), icon: <TableChart fontSize="small" /> },
        ]}
      />
    </>
  );
}

/** True when the only filter is one collection (the collection's own page). */
function onlyCollection(query: LibraryQuery): boolean {
  return query.labels.length === 1 && !hasFilters({ ...query, labels: [] });
}

interface LibraryEmptyProps {
  status: ReturnType<typeof useRows>['status'];
  /** Name of the collection whose page this is, when it has no other filters. */
  collection: string | null;
  onSearch: () => void;
  filtered: boolean;
  onRetry: () => void;
  onClear: () => void;
  onAdd: () => void;
}

function LibraryEmpty({ status, collection, onSearch, filtered, onRetry, onClear, onAdd }: LibraryEmptyProps) {
  const { t } = useTranslation();
  if (status === 'error') {
    return (
      <EmptyState
        title={t('library.empty.error', 'Documents could not be loaded')}
        action={<Button onPress={onRetry}>{t('library.retry', 'Try again')}</Button>}
      />
    );
  }
  if (collection) {
    return (
      <EmptyState
        title={t('library.empty.collection', { name: collection, defaultValue: 'Nothing in “{{name}}” yet' })}
        description={t('library.empty.collectionHint', 'Search for documents, tick the ones that belong here and choose “Save as collection”.')}
        action={<Button onPress={onSearch}>{t('library.empty.goSearch', 'Go to search')}</Button>}
      />
    );
  }
  if (filtered) {
    return (
      <EmptyState
        title={t('library.empty.noMatches', 'No matches')}
        description={t('library.empty.noMatchesHint', 'Nothing fits the current search and filters.')}
        action={<Button onPress={onClear}>{t('library.empty.clearFilters', 'Clear filters')}</Button>}
      />
    );
  }
  return (
    <EmptyState
      title={t('library.empty.none', 'No documents yet')}
      description={t('library.empty.noneHint', 'Upload files or connect a folder to start your library.')}
      action={
        <Button variant="primary" icon={<Add fontSize="small" />} onPress={onAdd}>
          {t('library.add', 'Add documents')}
        </Button>
      }
    />
  );
}

export default Library;
