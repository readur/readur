import { useTranslation } from 'react-i18next';
import { FilterChip, Select, SelectItem, Switch } from '../../ui';
import type { LabelData } from '../labels';
import type { LibrarySource } from './data';
import { ADDED_PRESETS, AddedPanel, LabelPanel, SourcePanel, STATUS_WORDS, StatusPanel, TypePanel, presetFrom } from './filterPanels';
import { TYPE_GROUP_LABELS } from './mime';
import { SearchBox } from './SearchBox';
import { SearchHelp } from './SearchHelp';
import { SORT_FIELDS, UPLOADED, hasFilters, isSearch, type LibraryQuery, type QueryPatch, type SortField } from './urlState';
import styles from './Library.module.css';

export const SORT_LABELS: Record<SortField, { key: string; fallback: string }> = {
  created_at: { key: 'library.columns.added', fallback: 'Added' },
  updated_at: { key: 'library.columns.updated', fallback: 'Updated' },
  filename: { key: 'library.columns.name', fallback: 'Name' },
  file_size: { key: 'library.columns.size', fallback: 'Size' },
  ocr_status: { key: 'library.columns.status', fallback: 'Status' },
  mime_type: { key: 'library.columns.type', fallback: 'Type' },
};

const RELEVANCE = 'relevance';

interface FilterStripProps {
  query: LibraryQuery;
  update: (patch: QueryPatch) => void;
  clearSort: () => void;
  clearFilters: () => void;
  labels: LabelData[];
  sources: LibrarySource[];
  compact: boolean;
  onCompactChange: (compact: boolean) => void;
}

/** "First, +2" for a chip value. */
function summarize(names: string[]): string | undefined {
  if (names.length === 0) return undefined;
  return names.length === 1 ? names[0] : `${names[0]} +${names.length - 1}`;
}

export function FilterStrip({
  query,
  update,
  clearSort,
  clearFilters,
  labels,
  sources,
  compact,
  onCompactChange,
}: FilterStripProps) {
  const { t } = useTranslation();

  const typeNames = query.types.map((g) => t(TYPE_GROUP_LABELS[g].key, TYPE_GROUP_LABELS[g].fallback));
  const labelNames = query.labels.map((id) => labels.find((l) => l.id === id)?.name ?? '…');
  const sourceNames = query.sources.map((id) =>
    id === UPLOADED ? t('library.source.upload', 'Upload') : (sources.find((s) => s.id === id)?.name ?? '…'),
  );
  const preset = ADDED_PRESETS.find((p) => !query.to && query.from === presetFrom(p.days));
  const addedValue = preset
    ? t(preset.key, preset.fallback)
    : query.from || query.to
      ? `${query.from ?? '…'} – ${query.to ?? '…'}`
      : undefined;

  const searching = isSearch(query);
  const sortKey = searching && !query.sortExplicit ? RELEVANCE : `${query.sort}:${query.order}`;
  const sortItems = [
    ...(searching ? [{ id: RELEVANCE, name: t('library.sort.relevance', 'Best match') }] : []),
    ...SORT_FIELDS.flatMap((f) =>
      (['desc', 'asc'] as const).map((o) => ({
        id: `${f}:${o}`,
        name: t(o === 'desc' ? 'library.sort.desc' : 'library.sort.asc', {
          field: t(SORT_LABELS[f].key, SORT_LABELS[f].fallback),
          defaultValue: o === 'desc' ? '{{field}}, descending' : '{{field}}, ascending',
        }),
      })),
    ),
  ];

  return (
    <div className={styles.strip} role="search" aria-label={t('library.filters.region', 'Search and filter')}>
      <div className={styles.searchGroup}>
        <SearchBox value={query.q} onChange={(q) => update({ q })} />
        <SearchHelp
          mode={query.mode}
          onModeChange={(mode) => update({ mode })}
          onExample={(q, mode) => update({ q, mode })}
        />
      </div>
      <div className={styles.chips}>
        <FilterChip
          label={t('library.filters.type', 'Type')}
          value={summarize(typeNames)}
          isActive={query.types.length > 0}
          onClear={() => update({ types: [] })}
          popover={<TypePanel value={query.types} onChange={(types) => update({ types })} />}
        />
        <FilterChip
          label={t('library.filters.label', 'Label')}
          value={summarize(labelNames)}
          isActive={query.labels.length > 0}
          onClear={() => update({ labels: [] })}
          popover={<LabelPanel labels={labels} value={query.labels} onChange={(ids) => update({ labels: ids })} />}
        />
        <FilterChip
          label={t('library.filters.status', 'Status')}
          value={query.status ? t(STATUS_WORDS[query.status].key, STATUS_WORDS[query.status].fallback) : undefined}
          isActive={query.status !== null}
          onClear={() => update({ status: null })}
          popover={<StatusPanel value={query.status} onChange={(status) => update({ status })} />}
        />
        <FilterChip
          label={t('library.filters.source', 'Source')}
          value={summarize(sourceNames)}
          isActive={query.sources.length > 0}
          onClear={() => update({ sources: [] })}
          popover={<SourcePanel sources={sources} value={query.sources} onChange={(ids) => update({ sources: ids })} />}
        />
        <FilterChip
          label={t('library.filters.added', 'Added')}
          value={addedValue}
          isActive={Boolean(query.from || query.to)}
          onClear={() => update({ from: null, to: null })}
          popover={<AddedPanel from={query.from} to={query.to} onChange={(range) => update(range)} />}
        />
        {hasFilters(query) ? (
          <button type="button" className={styles.clearAll} onClick={clearFilters}>
            {t('library.filters.clearAll', 'Clear all')}
          </button>
        ) : null}
      </div>
      <div className={styles.viewControls}>
        <Select
          className={styles.mobileSort}
          aria-label={t('library.sort.label', 'Sort by')}
          items={sortItems}
          selectedKey={sortKey}
          onSelectionChange={(key) => {
            const value = String(key);
            if (value === RELEVANCE) {
              clearSort();
              return;
            }
            const [sort, order] = value.split(':') as [SortField, 'asc' | 'desc'];
            update({ sort, order });
          }}
        >
          {(item) => <SelectItem id={item.id}>{item.name}</SelectItem>}
        </Select>
        <Switch
          isSelected={compact}
          onChange={onCompactChange}
          className={styles.density}
          label={t('library.compact', 'Compact rows')}
        />
      </div>
    </div>
  );
}
