import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation } from 'react-router-dom';
import { BulkActionBar, Button, Checkbox, EmptyState, Pagination } from '../../../ui';
import { BookmarkBorder, Schedule, Sort } from '../../../ui/icons';
import { PageHeader } from '../../shell';
import { BulkActions } from '../BulkActions';
import { fetchAllMatchIds } from '../data';
import { FilterStrip } from '../FilterStrip';
import { formatCount } from '../format';
import { monthLabel, monthRange } from '../months';
import { SearchBox } from '../SearchBox';
import { SearchHelp } from '../SearchHelp';
import { Segmented } from '../Segmented';
import { useFacets, useRows, useSourceName } from '../useLibraryData';
import { MIN_QUERY, NO_FILTERS, PAGE_SIZES, hasFilters, isSearch, useLibraryQuery } from '../urlState';
import { ResultList } from './ResultList';
import { useLibraryDrawer } from '../useLibraryDrawer';
import { SaveCollectionDialog } from './SaveCollectionDialog';
import { Timeline } from './Timeline';
import { useTimeline } from './useTimeline';
import styles from './Search.module.css';

type Order = 'date' | 'relevance';

/**
 * /search: one large field, how many documents match and when (a month histogram that narrows
 * the dates), filters, and the matches newest first under month headings with their passages.
 * Ticked results, or every match, can be saved as a collection.
 */
export function SearchPage() {
  const { t, i18n } = useTranslation();
  const location = useLocation();
  const { query, update, clearFilters } = useLibraryQuery();
  const facets = useFacets();
  const searching = isSearch(query);
  const { rows, total, status, reload, patchRow, mimes } = useRows(query, facets.mimeTypes, facets.mimeReady, searching);
  const timeline = useTimeline(query, mimes, searching && (query.types.length === 0 || facets.mimeReady));
  const sourceName = useSourceName(facets.sources);

  // Selection is per page of results; "every match" is a separate, explicit step.
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [allMatches, setAllMatches] = useState(false);
  const viewKey = JSON.stringify(query);
  useEffect(() => {
    setSelected(new Set());
    setAllMatches(false);
  }, [viewKey]);
  const toggle = useCallback((id: string, on: boolean) => {
    setAllMatches(false);
    setSelected((prev) => {
      const next = new Set(prev);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);
  const pageIds = rows.map((r) => r.id);
  const pageAllSelected = pageIds.length > 0 && pageIds.every((id) => selected.has(id));
  const clearSelection = () => {
    setSelected(new Set());
    setAllMatches(false);
  };

  const [saving, setSaving] = useState(false);
  const selectedRows = rows.filter((r) => selected.has(r.id));
  const saveCount = allMatches ? total : selectedRows.length;
  const resolveIds = useCallback(
    () => (allMatches ? fetchAllMatchIds(query, mimes, total) : Promise.resolve(selectedRows.map((r) => r.id))),
    [allMatches, query, mimes, total, selectedRows],
  );

  // Results open in the document drawer over this page; ↑/↓ walks the results.
  const drawer = useLibraryDrawer(rows, patchRow, reload);

  const pickMonths = (range: { start: string; end: string } | null) => {
    if (!range) {
      update({ from: null, to: null });
      return;
    }
    update({ from: monthRange(range.start)?.from ?? null, to: monthRange(range.end)?.to ?? null });
  };
  const pickedLabel = pickedMonths(query.from, query.to, i18n.language);

  const autoFocus = Boolean((location.state as { focusSearch?: boolean } | null)?.focusSearch) || !query.q;
  const byMonth = !query.relevance && query.sort === 'created_at';

  return (
    <div className={styles.page}>
      <PageHeader title={t('library.search.title', 'Advanced search')} />
      <div className={styles.searchRow}>
        <SearchBox
          value={query.q}
          onChange={(q) => update({ q })}
          autoFocus={autoFocus}
          size="lg"
          placeholder={t('library.search.bigPlaceholder', 'Search every document by name or by the words inside it')}
        />
        <SearchHelp mode={query.mode} onModeChange={(mode) => update({ mode })} onExample={(q, mode) => update({ q, mode })} />
      </div>
      <FilterStrip
        query={query}
        update={update}
        clearFilters={clearFilters}
        labels={facets.labels}
        sources={facets.sources}
        controls={
          searching ? (
            <Segmented<Order>
              label={t('library.search.order', 'Order')}
              value={query.relevance ? 'relevance' : 'date'}
              onChange={(v) => update({ relevance: v === 'relevance' })}
              items={[
                { id: 'date', label: t('library.search.newest', 'Newest first'), icon: <Schedule fontSize="small" /> },
                { id: 'relevance', label: t('library.search.bestMatch', 'Best match'), icon: <Sort fontSize="small" /> },
              ]}
            />
          ) : null
        }
      />

      {!searching ? (
        <SearchStart tooShort={query.q.trim().length > 0 && query.q.trim().length < MIN_QUERY} />
      ) : (
        <>
          <section className={styles.overview} aria-labelledby="search-summary">
            <div className={styles.summaryRow}>
              <p id="search-summary" className={styles.summary} aria-live="polite">
                {status === 'loading' && total === 0
                  ? t('library.search.counting', 'Searching…')
                  : status === 'ready'
                    ? t('library.search.count', {
                        count: total,
                        formatted: formatCount(total, i18n.language),
                        q: query.q.trim(),
                        defaultValue: '{{formatted}} documents match “{{q}}”',
                        defaultValue_one: '1 document matches “{{q}}”',
                      })
                    : null}
                {pickedLabel ? (
                  <span className={styles.summaryRange}>
                    {' '}
                    {t('library.search.inRange', { range: pickedLabel, defaultValue: 'in {{range}}' })}
                  </span>
                ) : null}
              </p>
              {query.from || query.to ? (
                <Button size="sm" variant="ghost" onPress={() => update({ from: null, to: null })}>
                  {t('library.search.allDates', 'Show all dates')}
                </Button>
              ) : null}
            </div>
            <Timeline bars={timeline.bars} from={query.from} to={query.to} onPick={pickMonths} isLoading={timeline.status === 'loading'} />
          </section>

          {rows.length > 0 ? (
            <div className={styles.selectRow}>
              <Checkbox
                label={t('library.search.selectPage', 'Select all on this page')}
                isSelected={pageAllSelected}
                isIndeterminate={!pageAllSelected && selected.size > 0}
                onChange={(on) => {
                  setAllMatches(false);
                  setSelected(on ? new Set(pageIds) : new Set());
                }}
              />
              {pageAllSelected && total > rows.length && !allMatches ? (
                <Button size="sm" variant="ghost" onPress={() => setAllMatches(true)}>
                  {t('library.search.selectAll', { count: total, formatted: formatCount(total, i18n.language), defaultValue: 'Select all {{formatted}} matches' })}
                </Button>
              ) : null}
              {allMatches ? (
                <span className={styles.allNote}>
                  {t('library.search.allSelected', { count: total, formatted: formatCount(total, i18n.language), defaultValue: 'All {{formatted}} matches are selected' })}
                </span>
              ) : null}
            </div>
          ) : null}

          <ResultList
            rows={status === 'ready' || status === 'loading' ? rows : []}
            byMonth={byMonth}
            selected={selected}
            onToggle={toggle}
            href={drawer.href}
            linkState={drawer.linkState}
            sourceName={sourceName}
            isLoading={status === 'loading'}
            emptyState={<SearchEmpty status={status} filtered={hasFilters(query)} onRetry={reload} onClear={() => update(NO_FILTERS)} />}
          />
          {total > 0 && status !== 'error' ? (
            <Pagination
              page={query.page}
              pageSize={query.size}
              total={total}
              pageSizeOptions={[...PAGE_SIZES]}
              onChange={(page, size) => update(size !== query.size ? { page: 1, size } : { page })}
            />
          ) : null}
        </>
      )}

      {allMatches ? (
        <BulkActionBar
          count={total}
          onClear={clearSelection}
          actions={[{ id: 'collection', label: t('library.collection.action', 'Save as collection'), icon: <BookmarkBorder fontSize="small" />, onPress: () => setSaving(true) }]}
        />
      ) : (
        <BulkActions
          selected={selectedRows}
          onClear={clearSelection}
          availableLabels={facets.labels}
          onLabelCreated={facets.addLabel}
          onChanged={reload}
          onSaveCollection={() => setSaving(true)}
        />
      )}
      <SaveCollectionDialog
        isOpen={saving}
        onOpenChange={setSaving}
        count={saveCount}
        labels={facets.labels}
        suggestedName={query.q}
        resolveIds={resolveIds}
        onSaved={(label) => {
          facets.addLabel(label);
          clearSelection();
          reload();
        }}
      />
    </div>
  );
}

/** "March 2024", or "Jan 2024 – Mar 2024" for a span of whole months; otherwise the dates. */
function pickedMonths(from: string | null, to: string | null, lng: string): string | null {
  if (!from && !to) return null;
  if (from && to) {
    const start = from.slice(0, 7);
    const end = to.slice(0, 7);
    const whole = monthRange(start)?.from === from && monthRange(end)?.to === to;
    if (whole) return start === end ? monthLabel(start, lng) : `${monthLabel(start, lng, 'short')} – ${monthLabel(end, lng, 'short')}`;
  }
  return `${from ?? '…'} – ${to ?? '…'}`;
}

function SearchStart({ tooShort }: { tooShort: boolean }) {
  const { t } = useTranslation();
  return (
    <div className={styles.start}>
      <EmptyState
        title={tooShort ? t('library.search.keepTyping', 'Keep typing…') : t('library.search.startTitle', 'Find anything in your documents')}
        description={t(
          'library.search.startHint',
          'Readur searches file names and every word read from your scans and PDFs. Try a name, a place or a topic, like “shoulder” or “invoice 2024”.',
        )}
      />
    </div>
  );
}

function SearchEmpty({ status, filtered, onRetry, onClear }: { status: string; filtered: boolean; onRetry: () => void; onClear: () => void }) {
  const { t } = useTranslation();
  if (status === 'tooMany') {
    return (
      <EmptyState
        title={t('library.empty.tooMany', 'Too many matches — refine your search')}
        description={t('library.empty.tooManyHint', 'Add a word or a filter to narrow the results.')}
      />
    );
  }
  if (status === 'error') {
    return (
      <EmptyState
        title={t('library.search.error', 'The search did not go through')}
        action={<Button onPress={onRetry}>{t('library.retry', 'Try again')}</Button>}
      />
    );
  }
  return (
    <EmptyState
      title={t('library.empty.noMatches', 'No matches')}
      description={
        filtered
          ? t('library.empty.noMatchesHint', 'Nothing fits the current search and filters.')
          : t('library.search.noMatchesHint', 'Check the spelling, try fewer words, or switch on “Similar spelling” in search help.')
      }
      action={filtered ? <Button onPress={onClear}>{t('library.empty.clearFilters', 'Clear filters')}</Button> : undefined}
    />
  );
}

export default SearchPage;
