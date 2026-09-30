import { Fragment, useId, useMemo, useRef, type CSSProperties, type ReactNode } from 'react';
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
import { useIsNarrow } from '../shared/useMediaQuery';
import { LoadingRows } from './LoadingRows';
import type { BoardColumn, BoardTableProps } from './types';
import { SELECT_CELL_ATTR, useRowPress } from './useRowPress';
import styles from './BoardTable.module.css';

const SELECT_COLUMN = '__select';

/**
 * React Aria's Row does not forward aria-describedby, so a ref links each row to its detail text.
 * The detail is aria-hidden inside the cell, which keeps it out of the row-header's name; only
 * the parts meant to be announced are referenced.
 */
function describedBy(ids: string[]) {
  return (el: HTMLElement | null) => {
    if (!el) return;
    if (ids.length > 0) el.setAttribute('aria-describedby', ids.join(' '));
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

function columnName<T>(col: BoardColumn<T>): string | undefined {
  return col.textValue ?? (typeof col.label === 'string' ? col.label : undefined);
}

function foldKind<T>(col: BoardColumn<T>): 'meta' | 'mark' | 'text' {
  return col.fold ?? (col.width === undefined ? 'text' : 'meta');
}

function isShown(value: ReactNode) {
  return value !== null && value !== undefined && value !== false && value !== '';
}

/**
 * The columns folded into a phone row. Visually: an optional tag and the marks, then one mono meta
 * line joined with middots, then any long text clamped to two lines. None of that is announced;
 * a visually hidden "Label: value;" list (`descId`) is the row's description instead.
 */
function FoldedFields<T>({
  columns,
  row,
  tag,
  descId,
}: {
  columns: BoardColumn<T>[];
  row: T;
  tag: ReactNode;
  descId: string;
}) {
  const marks: ReactNode[] = [];
  const metas: ReactNode[] = [];
  const texts: ReactNode[] = [];
  const ordered = columns
    .map((col, index) => ({ col, order: col.foldOrder ?? index }))
    .sort((a, b) => a.order - b.order)
    .map((entry) => entry.col);
  for (const col of ordered) {
    const value = col.foldValue ? col.foldValue(row) : col.render(row);
    if (!isShown(value)) continue;
    const kind = foldKind(col);
    if (kind === 'mark') marks.push(<span key={col.id} className={styles.foldMark}>{value}</span>);
    else if (kind === 'text') texts.push(<div key={col.id} className={styles.foldText}>{value}</div>);
    else metas.push(<span key={col.id} className={styles.metaItem}>{value}</span>);
  }
  const hasLine = isShown(tag) || marks.length > 0 || metas.length > 0;
  return (
    <>
      {hasLine ? (
        <div className={styles.foldLine}>
          {isShown(tag) ? <span className={styles.foldMark}>{tag}</span> : null}
          {marks}
          {metas.length > 0 ? (
            <span className={styles.meta}>
              {metas.map((m, i) => (
                <Fragment key={i}>
                  {i > 0 ? <span className={styles.metaSep}> · </span> : null}
                  {m}
                </Fragment>
              ))}
            </span>
          ) : null}
        </div>
      ) : null}
      {texts}
      {columns.length > 0 ? (
        <span id={descId} className="visually-hidden">
          {columns.map((col) => {
            const name = columnName(col);
            // The whitespace text nodes separate the pairs in the announced description.
            return (
              <Fragment key={col.id}>
                {name ? `${name}: ` : null}
                {col.render(row)};{' '}
              </Fragment>
            );
          })}
        </span>
      ) : null}
    </>
  );
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
  columns: allColumns,
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
  renderRowTag,
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
  // On a phone every row stacks: the name gets the full width and the rest folds under it.
  const narrow = useIsNarrow();
  const headerId = (allColumns.find((c) => c.isRowHeader) ?? allColumns[0])?.id;
  const columns = useMemo(
    () => (narrow ? allColumns.filter((c) => !c.hideOnNarrow || c.id === headerId) : allColumns),
    [allColumns, narrow, headerId],
  );
  const headerIndex = Math.max(0, columns.findIndex((c) => c.id === headerId));
  // On a phone the dropped columns are not lost: they fold into the first cell's detail line.
  const folded = useMemo(
    () => (narrow ? allColumns.filter((c) => c.hideOnNarrow && c.id !== headerId) : []),
    [allColumns, narrow, headerId],
  );
  const showSkeleton = isLoading && rows.length === 0;

  const sortDescriptor: SortDescriptor | undefined = sort
    ? { column: sort.column, direction: sort.direction }
    : undefined;

  return (
    <div
      ref={containerRef}
      className={cx(styles.container, className)}
      data-density={density}
      data-layout={narrow ? 'stacked' : undefined}
      data-selectable={multiple ? 'true' : undefined}
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
              textValue={columnName(col)}
              className={cx(styles.column, styles[`align-${col.align ?? 'start'}`], i === headerIndex && styles.headerColumn)}
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
                const custom = renderRowDetail?.(row);
                const hasCustom = isShown(custom);
                const tag = narrow ? renderRowTag?.(row) : null;
                const hasFold = folded.length > 0 || isShown(tag);
                const customId = hasCustom ? `${detailPrefix}-detail-${id}` : undefined;
                const descId = folded.length > 0 ? `${detailPrefix}-fields-${id}` : undefined;
                const detail =
                  hasCustom || hasFold ? (
                    <>
                      {hasFold ? <FoldedFields columns={folded} row={row} tag={tag} descId={descId ?? ''} /> : null}
                      {hasCustom ? (
                        <div id={customId} className={styles.detailText}>
                          {custom}
                        </div>
                      ) : null}
                    </>
                  ) : null;
                const described = [customId, descId].filter((x): x is string => Boolean(x));
                return (
                  <Row
                    key={id}
                    id={id}
                    className={cx(styles.row, changed && styles.changed, detail ? styles.hasDetail : undefined)}
                    data-changed={changed ? 'true' : undefined}
                    data-flip-key={id}
                    ref={describedBy(described)}
                  >
                    {multiple ? (
                      <Cell className={cx(styles.cell, styles.selectCell)} {...{ [SELECT_CELL_ATTR]: 'true' }}>
                        <Checkbox slot="selection" />
                      </Cell>
                    ) : null}
                    {columns.map((col, i) => (
                      <Cell
                        key={col.id}
                        className={cx(
                          styles.cell,
                          styles[`align-${col.align ?? 'start'}`],
                          isData(col) && styles.data,
                          i === headerIndex && styles.headerCell,
                        )}
                      >
                        <div className={styles.cellValue}>{col.render(row)}</div>
                        {i === headerIndex && detail ? (
                          <div className={styles.detail} aria-hidden="true">
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
