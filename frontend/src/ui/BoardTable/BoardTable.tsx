import { useId, useMemo, useRef, type CSSProperties } from 'react';
import {
  Cell,
  Column,
  Row,
  Table,
  TableBody,
  TableHeader,
  type SortDescriptor,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Checkbox } from '../Checkbox';
import { useFlip } from '../motion';
import { cx } from '../shared/FieldParts';
import { LoadingRows } from './LoadingRows';
import type { BoardColumn, BoardTableProps } from './types';
import { SELECT_CELL_ATTR, useRowPress } from './useRowPress';
import styles from './BoardTable.module.css';

const SELECT_COLUMN = '__select';

/**
 * React Aria's Row does not forward aria-describedby, so a ref links each row to its detail line.
 * The detail is aria-hidden inside the cell, which keeps it out of the row-header's name.
 */
function describedBy(detailId: string | undefined) {
  return (el: HTMLElement | null) => {
    if (!el) return;
    if (detailId) el.setAttribute('aria-describedby', detailId);
    else el.removeAttribute('aria-describedby');
  };
}

function widthStyle(width: number | string | undefined): CSSProperties | undefined {
  if (width === undefined) return undefined;
  return { width: typeof width === 'number' ? `${width}px` : width };
}

function isData<T>(col: BoardColumn<T>) {
  return col.mono || col.align === 'end';
}

/**
 * Dense sortable, selectable table built on the React Aria Table. Sorting and selection are
 * controlled by the parent. Rows reorder with a FLIP animation.
 *
 * Interaction model: a row click, tap or Enter always runs `onRowAction`, even while rows are
 * selected. Selection changes only through the checkbox cells, Space on a focused row and the
 * select-all checkbox.
 */
export function BoardTable<T>({
  columns,
  rows,
  getRowId,
  sort,
  onSortChange,
  selectionMode = 'none',
  selectedKeys,
  onSelectionChange,
  onRowAction,
  isRowLit,
  renderRowDetail,
  density = 'comfortable',
  isLoading = false,
  loadingRowCount = 8,
  emptyState,
  className,
  'aria-label': ariaLabel,
  'aria-labelledby': ariaLabelledBy,
}: BoardTableProps<T>) {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const ids = useMemo(() => rows.map(getRowId), [rows, getRowId]);
  useFlip(ids, containerRef);
  const pressHandlers = useRowPress(containerRef, onRowAction);
  const detailPrefix = useId();

  const multiple = selectionMode === 'multiple';
  const headerIndex = Math.max(0, columns.findIndex((c) => c.isRowHeader));
  const showSkeleton = isLoading && rows.length === 0;

  const sortDescriptor: SortDescriptor | undefined = sort
    ? { column: sort.column, direction: sort.direction }
    : undefined;

  return (
    <div
      ref={containerRef}
      className={cx(styles.container, className)}
      data-density={density}
      aria-busy={isLoading || undefined}
      {...pressHandlers}
    >
      <Table
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        className={cx(styles.table, onRowAction && styles.actionable)}
        selectionMode={multiple ? 'multiple' : 'none'}
        selectionBehavior="toggle"
        selectedKeys={selectedKeys}
        onSelectionChange={onSelectionChange}
        sortDescriptor={sortDescriptor}
        onSortChange={(d) => onSortChange?.({ column: String(d.column), direction: d.direction })}
      >
        <TableHeader className={styles.head}>
          {multiple ? (
            <Column id={SELECT_COLUMN} className={cx(styles.column, styles.selectColumn)}>
              <Checkbox slot="selection" />
            </Column>
          ) : null}
          {columns.map((col, i) => (
            <Column
              key={col.id}
              id={col.id}
              isRowHeader={i === headerIndex}
              allowsSorting={Boolean(col.sortable)}
              textValue={col.textValue ?? (typeof col.label === 'string' ? col.label : undefined)}
              className={cx(styles.column, styles[`align-${col.align ?? 'start'}`])}
              style={widthStyle(col.width)}
            >
              {({ allowsSorting, sortDirection }) => (
                <span className={styles.columnInner}>
                  <span className={styles.columnLabel}>{col.label}</span>
                  {allowsSorting ? (
                    <span className={cx(styles.sortMark, sortDirection && styles.sorted)} aria-hidden="true">
                      {sortDirection === 'descending' ? '▼' : '▲'}
                    </span>
                  ) : null}
                </span>
              )}
            </Column>
          ))}
        </TableHeader>
        <TableBody
          className={styles.body}
          renderEmptyState={() =>
            showSkeleton ? (
              <LoadingRows count={loadingRowCount} label={t('ui.board.loading', 'Loading')} />
            ) : (
              <div className={styles.empty}>{emptyState ?? t('ui.board.empty', 'Nothing to show')}</div>
            )
          }
        >
          {showSkeleton
            ? []
            : rows.map((row, rowIndex) => {
                const id = ids[rowIndex];
                const changed = isRowLit?.(row) ?? false;
                const detail = renderRowDetail?.(row);
                const detailId = detail ? `${detailPrefix}-detail-${id}` : undefined;
                return (
                  <Row
                    key={id}
                    id={id}
                    className={cx(styles.row, changed && styles.changed, detail ? styles.hasDetail : undefined)}
                    data-changed={changed ? 'true' : undefined}
                    data-flip-key={id}
                    ref={describedBy(detailId)}
                  >
                    {multiple ? (
                      <Cell className={cx(styles.cell, styles.selectCell)} {...{ [SELECT_CELL_ATTR]: 'true' }}>
                        <Checkbox slot="selection" />
                      </Cell>
                    ) : null}
                    {columns.map((col, i) => (
                      <Cell
                        key={col.id}
                        className={cx(styles.cell, styles[`align-${col.align ?? 'start'}`], isData(col) && styles.data)}
                      >
                        <div className={styles.cellValue}>{col.render(row)}</div>
                        {i === 0 && detail ? (
                          <div id={detailId} className={styles.detail} aria-hidden="true">
                            {detail}
                          </div>
                        ) : null}
                      </Cell>
                    ))}
                  </Row>
                );
              })}
        </TableBody>
      </Table>
    </div>
  );
}
