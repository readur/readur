import { useMemo, useRef, type CSSProperties } from 'react';
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
import styles from './BoardTable.module.css';

const SELECT_COLUMN = '__select';

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
        onRowAction={onRowAction ? (key) => onRowAction(String(key)) : undefined}
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
                const lit = isRowLit?.(row) ?? false;
                const detail = renderRowDetail?.(row);
                return (
                  <Row
                    key={id}
                    id={id}
                    className={cx(styles.row, lit && styles.lit, detail ? styles.hasDetail : undefined)}
                    data-lit={lit ? 'true' : undefined}
                    data-flip-key={id}
                  >
                    {multiple ? (
                      <Cell className={cx(styles.cell, styles.selectCell)}>
                        <Checkbox slot="selection" />
                      </Cell>
                    ) : null}
                    {columns.map((col, i) => (
                      <Cell
                        key={col.id}
                        className={cx(styles.cell, styles[`align-${col.align ?? 'start'}`], isData(col) && styles.data)}
                      >
                        <div className={styles.cellValue}>{col.render(row)}</div>
                        {i === 0 && detail ? <div className={styles.detail}>{detail}</div> : null}
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
