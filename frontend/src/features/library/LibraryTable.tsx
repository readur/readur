import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, type BoardColumn, type BoardSort, type Selection } from '../../ui';
import { isLit } from '../board/litStore';
import { AddedCell, LabelsCell, NameCell, StatusCell } from './cells';
import type { LibraryRow } from './data';
import { formatBytes } from './format';
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
      { id: 'mime_type', label: t('library.columns.type', 'Type'), sortable: true, width: 72, mono: true, render: (r) => shortType(r.mime_type) },
      { id: 'ocr_status', label: t('library.columns.status', 'Status'), sortable: true, width: 128, render: (r) => <StatusCell row={r} /> },
      { id: 'source', label: t('library.columns.source', 'Source'), width: 140, render: (r) => sourceName(r) },
      { id: 'labels', label: t('library.columns.labels', 'Labels'), width: 200, render: (r) => <LabelsCell row={r} /> },
      {
        id: 'file_size',
        label: t('library.columns.size', 'Size'),
        sortable: true,
        width: 88,
        align: 'end',
        render: (r) => formatBytes(r.file_size, i18n.language),
      },
      { id: 'created_at', label: t('library.columns.added', 'Added'), sortable: true, width: 104, mono: true, render: (r) => <AddedCell value={r.created_at} /> },
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
      density={compact ? 'compact' : 'comfortable'}
      isLoading={isLoading}
      emptyState={emptyState}
    />
  );
}
