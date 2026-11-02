import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { FilterChip, Select, SelectItem } from '../../ui';
import type { LabelData } from '../labels';
import type { LibrarySource } from './data';
import { ADDED_PRESETS, AddedPanel, LabelPanel, SourcePanel, STATUS_WORDS, StatusPanel, TypePanel, presetFrom } from './filterPanels';
import { TYPE_GROUP_LABELS } from './mime';
import { SORT_FIELDS, UPLOADED, WATCHED, hasFilters, type LibraryQuery, type QueryPatch, type SortField } from './urlState';
import styles from './Library.module.css';

export const SORT_LABELS: Record<SortField, { key: string; fallback: string }> = {
  created_at: { key: 'library.columns.added', fallback: 'Added' },
  updated_at: { key: 'library.columns.updated', fallback: 'Updated' },
  filename: { key: 'library.columns.name', fallback: 'Name' },
  file_size: { key: 'library.columns.size', fallback: 'Size' },
  ocr_status: { key: 'library.columns.status', fallback: 'Status' },
  mime_type: { key: 'library.columns.type', fallback: 'Type' },
};

interface FilterStripProps {
  query: LibraryQuery;
  update: (patch: QueryPatch) => void;
  clearFilters: () => void;
  labels: LabelData[];
  sources: LibrarySource[];
  /** The search field (and its help), at the start of the strip. */
  search?: ReactNode;
  /** View controls at the end of the strip (order, layout, density). */
  controls?: ReactNode;
}

/** "First, +2" for a chip value. */
function summarize(names: string[]): string | undefined {
  if (names.length === 0) return undefined;
  return names.length === 1 ? names[0] : `${names[0]} +${names.length - 1}`;
}

/** Search, the filter chips (type, collection, status, source, added) and view controls. */
export function FilterStrip({ query, update, clearFilters, labels, sources, search, controls }: FilterStripProps) {
  const { t } = useTranslation();

  const typeNames = query.types.map((g) => t(TYPE_GROUP_LABELS[g].key, TYPE_GROUP_LABELS[g].fallback));
  const labelNames = query.labels.map((id) => labels.find((l) => l.id === id)?.name ?? '…');
  const sourceNames = query.sources.map((id) =>
    id === UPLOADED
      ? t('library.source.upload', 'Upload')
      : id === WATCHED
        ? t('library.source.watch', 'Watch folder')
        : (sources.find((s) => s.id === id)?.name ?? '…'),
  );
  const preset = ADDED_PRESETS.find((p) => !query.to && query.from === presetFrom(p.days));
  const addedValue = preset
    ? t(preset.key, preset.fallback)
    : query.from || query.to
      ? `${query.from ?? '…'} – ${query.to ?? '…'}`
      : undefined;

  return (
    <div className={styles.strip} role="search" aria-label={t('library.filters.region', 'Search and filter')}>
      {search ? <div className={styles.searchGroup}>{search}</div> : null}
      <div className={styles.chips}>
        <FilterChip
          label={t('library.filters.type', 'Type')}
          value={summarize(typeNames)}
          isActive={query.types.length > 0}
          onClear={() => update({ types: [] })}
          popover={<TypePanel value={query.types} onChange={(types) => update({ types })} />}
        />
        <FilterChip
          label={t('library.filters.collection', 'Collection')}
          value={summarize(labelNames)}
          isActive={query.labels.length > 0}
          onClear={() => update({ labels: [] })}
          popover={<LabelPanel labels={labels} value={query.labels} onChange={(ids) => update({ labels: ids })} />}
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
        <FilterChip
          label={t('library.filters.status', 'Status')}
          value={query.status ? t(STATUS_WORDS[query.status].key, STATUS_WORDS[query.status].fallback) : undefined}
          isActive={query.status !== null}
          onClear={() => update({ status: null })}
          popover={<StatusPanel value={query.status} onChange={(status) => update({ status })} />}
        />
        {hasFilters(query) ? (
          <button type="button" className={styles.clearAll} onClick={clearFilters}>
            {t('library.filters.clearAll', 'Clear all')}
          </button>
        ) : null}
      </div>
      {controls ? <div className={styles.viewControls}>{controls}</div> : null}
    </div>
  );
}

/** Field and direction in one menu, for screens where the table's column heads are not shown. */
export function SortSelect({ query, update, className }: { query: LibraryQuery; update: (patch: QueryPatch) => void; className?: string }) {
  const { t } = useTranslation();
  const items = SORT_FIELDS.flatMap((f) =>
    (['desc', 'asc'] as const).map((o) => ({
      id: `${f}:${o}`,
      name: t(o === 'desc' ? 'library.sort.desc' : 'library.sort.asc', {
        field: t(SORT_LABELS[f].key, SORT_LABELS[f].fallback),
        defaultValue: o === 'desc' ? '{{field}}, descending' : '{{field}}, ascending',
      }),
    })),
  );
  return (
    <Select
      className={className}
      aria-label={t('library.sort.label', 'Sort by')}
      items={items}
      selectedKey={`${query.sort}:${query.order}`}
      onSelectionChange={(key) => {
        const [sort, order] = String(key).split(':') as [SortField, 'asc' | 'desc'];
        update({ sort, order });
      }}
    >
      {(item) => <SelectItem id={item.id}>{item.name}</SelectItem>}
    </Select>
  );
}
