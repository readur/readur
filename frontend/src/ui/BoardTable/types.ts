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
  /**
   * Low-priority column: below 720px it leaves the column heads and folds into the row (see
   * `fold`). The row-header column is always kept; other kept columns flow on their own line
   * under it, so a phone never scrolls sideways.
   */
  hideOnNarrow?: boolean;
  /**
   * How a `hideOnNarrow` column shows inside the row on a phone:
   * - `meta` joins the row's mono meta line (`DOC · 2.0 KB · 3 min ago`), unlabelled;
   * - `mark` leads that line, unlabelled and unseparated (status marks);
   * - `text` gets its own line clamped to two lines (a failure reason, a description).
   * Defaults to `text` for columns without a width, `meta` otherwise. Every folded column is
   * still announced with its label as part of the row's accessible description.
   */
  fold?: 'meta' | 'mark' | 'text';
  /** Short value for the folded line on a phone; return null to leave it out. Defaults to `render`. */
  foldValue?: (row: T) => ReactNode;
  /**
   * Position on the folded meta line (lower first; default: column order). The line ends in an
   * ellipsis when it is too long, so put the values that matter most first.
   */
  foldOrder?: number;
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
  /** `multiple` adds a checkbox column and a select-all checkbox. Space on a focused row toggles it. */
  selectionMode?: 'none' | 'multiple';
  selectedKeys?: Selection;
  onSelectionChange?: (keys: Selection) => void;
  /**
   * Fires on click, tap or Enter on a row, including while other rows are selected. Presses on
   * the checkbox cell and on controls rendered inside cells do not fire it.
   */
  onRowAction?: (id: string) => void;
  /**
   * Rows that changed and still need the user's attention get a 3px edge bar and
   * `data-changed="true"`. The bar is colour only, so the consumer must also render a visible
   * NEW or CHANGED tag in one of the row's cells.
   */
  isRowLit?: (row: T) => boolean;
  /**
   * Optional second line (for example a search snippet) shown under the first column's value.
   * It is hidden from the row-header cell's accessible name and exposed instead as the row's
   * accessible description (`aria-describedby`).
   */
  renderRowDetail?: (row: T) => ReactNode;
  /**
   * The row's NEW / CHANGED tag, for phones: shown at the start of the row's second line, ahead of
   * the folded status and meta. It is visual only there (aria-hidden), so keep the tag in a cell
   * for wide screens and assistive tech, and hide it visually on phones.
   */
  renderRowTag?: (row: T) => ReactNode;
  density?: 'comfortable' | 'compact';
  /** With no rows, shows skeleton rows. With rows, keeps them visible and marks the table busy. */
  isLoading?: boolean;
  /** Number of skeleton rows while loading with no data. Default 8. */
  loadingRowCount?: number;
  /** Shown when `rows` is empty and not loading. */
  emptyState?: ReactNode;
  className?: string;
}
