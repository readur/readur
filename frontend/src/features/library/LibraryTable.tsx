import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, type BoardColumn, type BoardSort, type Selection } from '../../ui';
import { isLit } from '../board/litStore';
import { AddedCell, ChangeTag, LabelsCell, NameCell, StatusCell } from './cells';
import type { LibraryRow } from './data';
import { formatBytes, formatRelative } from './format';
import { SnippetLine, bestSnippet } from './Highlight';
import { shortType } from './mime';
import { SORT_FIELDS, type SortField } from './urlState';

interface LibraryTableProps {
  rows: LibraryRow[];
  sort: BoardSort | undefined;
  onSortChange: (field: SortField, order: 'asc' | 'desc') => void;
  selectedKeys: Selection;
  onSelectionChange: (keys: Selection) => void;
  onOpen: (id: string) => void;
  sourceName: (row: LibraryRow) => string;
  showSnippets: boolean;
  compact: boolean;
  isLoading: boolean;
  emptyState: React.ReactNode;
  /** Changes whenever the change-tracking store does, so edge bars stay current. */
  litVersion: number;
}

const getRowId = (row: LibraryRow) => row.id;
const renderRowTag = (row: LibraryRow) => <ChangeTag id={row.id} />;

export function LibraryTable({
  rows,
  sort,
  onSortChange,
  selectedKeys,
  onSelectionChange,
  onOpen,
  sourceName,
  showSnippets,
  compact,
  isLoading,
  emptyState,
  litVersion,
}: LibraryTableProps) {
  const { t, i18n } = useTranslation();

  const columns = useMemo<BoardColumn<LibraryRow>[]>(
    () => [
      { id: 'filename', label: t('library.columns.name', 'Name'), sortable: true, render: (r) => <NameCell row={r} /> },
      { id: 'mime_type', hideOnNarrow: true, label: t('library.columns.type', 'Type'), sortable: true, width: 72, mono: true, render: (r) => shortType(r.mime_type) },
      {
        id: 'ocr_status',
        hideOnNarrow: true,
        fold: 'mark',
        label: t('library.columns.status', 'Status'),
        sortable: true,
        width: 128,
        render: (r) => <StatusCell row={r} />,
      },
      {
        id: 'source',
        hideOnNarrow: true,
        label: t('library.columns.source', 'Source'),
        width: 140,
        render: (r) => sourceName(r),
        // After size and date on a phone's meta line, which truncates from the end.
        foldOrder: 10,
      },
      {
        id: 'labels',
        hideOnNarrow: true,
        label: t('library.columns.labels', 'Labels'),
        width: 200,
        render: (r) => <LabelsCell row={r} />,
        // On a phone the meta line names the labels as text, and leaves them out when there are none.
        foldValue: (r) => (r.labels.length > 0 ? r.labels.map((l) => l.name).join(', ') : null),
        foldOrder: 11,
      },
      {
        id: 'file_size',
        hideOnNarrow: true,
        label: t('library.columns.size', 'Size'),
        sortable: true,
        width: 88,
        align: 'end',
        render: (r) => formatBytes(r.file_size, i18n.language),
      },
      {
        id: 'created_at',
        hideOnNarrow: true,
        label: t('library.columns.added', 'Added'),
        sortable: true,
        width: 104,
        mono: true,
        render: (r) => <AddedCell value={r.created_at} />,
        foldValue: (r) => formatRelative(r.created_at, i18n.language),
      },
    ],
    [t, i18n.language, sourceName],
  );

  const isRowLit = useMemo(() => {
    void litVersion;
    return (row: LibraryRow) => isLit('document', row.id);
  }, [litVersion]);

  const renderRowDetail = showSnippets
    ? (row: LibraryRow) => {
        const snippet = bestSnippet(row.snippets);
        return snippet ? <SnippetLine snippet={snippet} /> : null;
      }
    : undefined;

  return (
    <BoardTable
      aria-label={t('library.table', 'Documents')}
      columns={columns}
      rows={rows}
      getRowId={getRowId}
      sort={sort}
      onSortChange={(s) => {
        if ((SORT_FIELDS as readonly string[]).includes(s.column)) {
          onSortChange(s.column as SortField, s.direction === 'ascending' ? 'asc' : 'desc');
        }
      }}
      selectionMode="multiple"
      selectedKeys={selectedKeys}
      onSelectionChange={onSelectionChange}
      onRowAction={onOpen}
      isRowLit={isRowLit}
      renderRowDetail={renderRowDetail}
      renderRowTag={renderRowTag}
      density={compact ? 'compact' : 'comfortable'}
      isLoading={isLoading}
      emptyState={emptyState}
    />
  );
}
