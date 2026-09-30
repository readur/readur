import type { ReactNode } from 'react';
import type { Selection } from 'react-aria-components';

export type { Selection };

export interface BoardColumn<T> {
  id: string;
  /** Column heading. Use a string where possible; it is also the sort button's name. */
  label: ReactNode;
  /** Plain-text heading when `label` is not a string. */
  textValue?: string;
  sortable?: boolean;
  /** CSS width (number = px). Columns without a width share the remaining space. */
  width?: number | string;
  align?: 'start' | 'center' | 'end';
  /** Render values in the data face with tabular numbers. Implied by `align: 'end'`. */
  mono?: boolean;
  /** Marks the column that names the row for assistive tech. Defaults to the first column. */
  isRowHeader?: boolean;
  render: (row: T) => ReactNode;
}

export interface BoardSort {
  column: string;
  direction: 'ascending' | 'descending';
}

export interface BoardTableProps<T> {
  columns: BoardColumn<T>[];
  rows: T[];
  getRowId: (row: T) => string;
  /** Accessible name of the table. Provide this or `aria-labelledby`. */
  'aria-label'?: string;
  'aria-labelledby'?: string;
  /** Controlled sort. Clicking a sortable head calls `onSortChange` with the toggled direction. */
  sort?: BoardSort;
  onSortChange?: (sort: BoardSort) => void;
  selectionMode?: 'none' | 'multiple';
  selectedKeys?: Selection;
  onSelectionChange?: (keys: Selection) => void;
  /** Fires on Enter or click on a row. */
  onRowAction?: (id: string) => void;
  /** Rows that changed and still need the user's attention get an edge bar and `data-lit`. */
  isRowLit?: (row: T) => boolean;
  /** Optional second line (for example a search snippet) under the row's first column. */
  renderRowDetail?: (row: T) => ReactNode;
  density?: 'comfortable' | 'compact';
  /** With no rows, shows skeleton rows. With rows, keeps them visible and marks the table busy. */
  isLoading?: boolean;
  /** Number of skeleton rows while loading with no data. Default 8. */
  loadingRowCount?: number;
  /** Shown when `rows` is empty and not loading. */
  emptyState?: ReactNode;
  className?: string;
}
