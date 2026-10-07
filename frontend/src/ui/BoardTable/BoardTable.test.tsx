import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { useState } from 'react';
import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createResponsiveMatchMediaMock } from '../../test/pwa-test-utils';
import { EmptyState } from '../EmptyState';
import { BoardTable } from './BoardTable';
import type { BoardColumn, BoardSort, BoardTableProps, Selection } from './types';

interface Doc {
  id: string;
  name: string;
  pages: number;
  isNew?: boolean;
  snippet?: string;
}

const DOCS: Doc[] = [
  { id: 'a', name: 'Alpha.pdf', pages: 3 },
  { id: 'b', name: 'Bravo.pdf', pages: 12, isNew: true, snippet: 'matched invoice total' },
  { id: 'c', name: 'Charlie.pdf', pages: 1 },
];

const COLUMNS: BoardColumn<Doc>[] = [
  { id: 'name', label: 'Name', sortable: true, render: (d) => d.name },
  { id: 'pages', label: 'Pages', sortable: true, align: 'end', render: (d) => d.pages },
  { id: 'type', label: 'Type', render: () => 'PDF' },
];

const getRowId = (d: Doc) => d.id;

function Harness(props: Partial<BoardTableProps<Doc>> & { initialSort?: BoardSort; onSort?: (s: BoardSort) => void }) {
  const { initialSort, onSort, ...rest } = props;
  const [sort, setSort] = useState<BoardSort | undefined>(initialSort);
  return (
    <BoardTable<Doc>
      aria-label="Documents"
      columns={COLUMNS}
      rows={DOCS}
      getRowId={getRowId}
      sort={sort}
      onSortChange={(s) => {
        onSort?.(s);
        setSort(s);
      }}
      {...rest}
    />
  );
}

function SelectHarness({ onChange, onRowAction }: { onChange: (keys: Selection) => void; onRowAction?: (id: string) => void }) {
  const [keys, setKeys] = useState<Selection>(new Set());
  return (
    <Harness
      selectionMode="multiple"
      onRowAction={onRowAction}
      selectedKeys={keys}
      onSelectionChange={(k) => {
        onChange(k);
        setKeys(k);
      }}
    />
  );
}

const header = (name: string) => screen.getByRole('columnheader', { name });

describe('BoardTable', () => {
  it('renders a named grid with heads and rows', () => {
    render(<Harness />);
    const grid = screen.getByRole('grid', { name: 'Documents' });
    expect(within(grid).getAllByRole('columnheader').map((h) => h.textContent)).toEqual(['Name▲', 'Pages▲', 'Type']);
    expect(within(grid).getAllByRole('row')).toHaveLength(4);
    expect(screen.getByRole('rowheader', { name: 'Alpha.pdf' })).toBeInTheDocument();
  });

  it('sorts by clicking a sortable head and toggles the direction', async () => {
    const onSort = vi.fn();
    const user = userEvent.setup();
    render(<Harness onSort={onSort} initialSort={{ column: 'name', direction: 'ascending' }} />);
    expect(header('Name')).toHaveAttribute('aria-sort', 'ascending');
    await user.click(header('Name'));
    expect(onSort).toHaveBeenLastCalledWith({ column: 'name', direction: 'descending' });
    expect(header('Name')).toHaveAttribute('aria-sort', 'descending');
    expect(header('Name')).toHaveTextContent('Name▼');
    await user.click(header('Pages'));
    expect(onSort).toHaveBeenLastCalledWith({ column: 'pages', direction: 'ascending' });
    expect(header('Pages')).toHaveAttribute('aria-sort', 'ascending');
    expect(header('Name')).toHaveAttribute('aria-sort', 'none');
  });

  it('sorts from the keyboard with Enter on a head', async () => {
    const onSort = vi.fn();
    const user = userEvent.setup();
    render(<Harness onSort={onSort} initialSort={{ column: 'pages', direction: 'descending' }} />);
    await user.tab();
    // Focus lands in the grid; move up into the header row, then onto the Pages head.
    await user.keyboard('{ArrowUp}');
    await user.keyboard('{ArrowRight}');
    expect(header('Pages')).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onSort).toHaveBeenLastCalledWith({ column: 'pages', direction: 'ascending' });
    expect(header('Pages')).toHaveAttribute('aria-sort', 'ascending');
  });

  it('does not sort non-sortable heads', async () => {
    const onSort = vi.fn();
    const user = userEvent.setup();
    render(<Harness onSort={onSort} />);
    expect(header('Type')).not.toHaveAttribute('aria-sort');
    await user.click(header('Type'));
    expect(onSort).not.toHaveBeenCalled();
  });

  it('fires onRowAction on Enter and on click', async () => {
    const onRowAction = vi.fn();
    const user = userEvent.setup();
    render(<Harness onRowAction={onRowAction} />);
    await user.tab();
    await user.keyboard('{ArrowDown}');
    await user.keyboard('{Enter}');
    expect(onRowAction).toHaveBeenLastCalledWith('b');
    await user.click(screen.getByRole('rowheader', { name: 'Charlie.pdf' }));
    expect(onRowAction).toHaveBeenLastCalledWith('c');
  });

  it('opens a row once when Enter is held down', async () => {
    const onRowAction = vi.fn();
    const user = userEvent.setup();
    render(<Harness onRowAction={onRowAction} />);
    await user.tab();
    await user.keyboard('{ArrowDown}');
    await user.keyboard('{Enter>3/}');
    expect(onRowAction).toHaveBeenCalledTimes(1);
    expect(onRowAction).toHaveBeenCalledWith('b');
  });

  it('selects rows with Space and the checkbox, and all rows from the header', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SelectHarness onChange={onChange} />);
    const rows = screen.getAllByRole('row').slice(1);
    await user.tab();
    await user.keyboard(' ');
    expect(rows[0]).toHaveAttribute('aria-selected', 'true');
    expect(within(rows[0]).getByRole('checkbox')).toBeChecked();

    await user.click(within(rows[2]).getByRole('checkbox'));
    expect(rows[2]).toHaveAttribute('aria-selected', 'true');
    expect(rows[1]).toHaveAttribute('aria-selected', 'false');

    const selectAll = screen.getByRole('checkbox', { name: /select all/i });
    await user.click(selectAll);
    expect(onChange).toHaveBeenLastCalledWith('all');
    for (const row of rows) expect(row).toHaveAttribute('aria-selected', 'true');
  });

  describe('while rows are selected', () => {
    async function selectFirstRow(user: ReturnType<typeof userEvent.setup>) {
      const rows = screen.getAllByRole('row').slice(1);
      await user.click(within(rows[0]).getByRole('checkbox'));
      expect(rows[0]).toHaveAttribute('aria-selected', 'true');
      return rows;
    }

    it('a click on another row opens it without changing the selection', async () => {
      const onRowAction = vi.fn();
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<SelectHarness onChange={onChange} onRowAction={onRowAction} />);
      const rows = await selectFirstRow(user);
      onChange.mockClear();
      await user.click(screen.getByRole('rowheader', { name: 'Charlie.pdf' }));
      expect(onRowAction).toHaveBeenCalledTimes(1);
      expect(onRowAction).toHaveBeenCalledWith('c');
      expect(onChange).not.toHaveBeenCalled();
      expect(rows[0]).toHaveAttribute('aria-selected', 'true');
      expect(rows[2]).toHaveAttribute('aria-selected', 'false');
      // Clicking a non-header cell of a selected row also opens it and keeps it selected.
      await user.click(within(rows[0]).getByRole('gridcell', { name: '3' }));
      expect(onRowAction).toHaveBeenLastCalledWith('a');
      expect(rows[0]).toHaveAttribute('aria-selected', 'true');
    });

    it('Enter on a row opens it without changing the selection', async () => {
      const onRowAction = vi.fn();
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<SelectHarness onChange={onChange} onRowAction={onRowAction} />);
      const rows = await selectFirstRow(user);
      onChange.mockClear();
      act(() => rows[1].focus());
      await user.keyboard('{Enter}');
      expect(onRowAction).toHaveBeenCalledWith('b');
      expect(onChange).not.toHaveBeenCalled();
      expect(rows[1]).toHaveAttribute('aria-selected', 'false');
      expect(rows[0]).toHaveAttribute('aria-selected', 'true');
    });

    it('checkbox, Space and select-all still change the selection', async () => {
      const onRowAction = vi.fn();
      const onChange = vi.fn();
      const user = userEvent.setup();
      render(<SelectHarness onChange={onChange} onRowAction={onRowAction} />);
      const rows = await selectFirstRow(user);
      await user.click(within(rows[1]).getByRole('checkbox'));
      expect(rows[1]).toHaveAttribute('aria-selected', 'true');
      act(() => rows[2].focus());
      await user.keyboard(' ');
      expect(rows[2]).toHaveAttribute('aria-selected', 'true');
      await user.keyboard(' ');
      expect(rows[2]).toHaveAttribute('aria-selected', 'false');
      await user.click(screen.getByRole('checkbox', { name: /select all/i }));
      expect(onChange).toHaveBeenLastCalledWith('all');
      for (const row of rows) expect(row).toHaveAttribute('aria-selected', 'true');
      expect(onRowAction).not.toHaveBeenCalled();
    });
  });

  it('a click on a row with no selection opens it and does not select it', async () => {
    const onRowAction = vi.fn();
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<SelectHarness onChange={onChange} onRowAction={onRowAction} />);
    await user.click(screen.getByRole('rowheader', { name: 'Bravo.pdf' }));
    expect(onRowAction).toHaveBeenCalledWith('b');
    expect(onChange).not.toHaveBeenCalled();
  });

  it('has no checkbox column without multiple selection', () => {
    render(<Harness />);
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
  });

  it('marks changed rows with data-changed', () => {
    render(<Harness isRowLit={(d) => Boolean(d.isNew)} />);
    const bravo = screen.getByRole('rowheader', { name: /Bravo\.pdf/ }).closest('[role="row"]');
    const alpha = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]');
    expect(bravo).toHaveAttribute('data-changed', 'true');
    expect(alpha).not.toHaveAttribute('data-changed');
    expect(bravo).not.toHaveAttribute('data-lit');
    // Rows carry their id for the reorder animation.
    expect(bravo).toHaveAttribute('data-flip-key', 'b');
  });

  it('renders a detail line inside the first cell as the row description, not its name', () => {
    render(<Harness renderRowDetail={(d) => d.snippet} />);
    const cell = screen.getByRole('rowheader', { name: 'Bravo.pdf' });
    expect(within(cell).getByText('matched invoice total')).toBeInTheDocument();
    const row = cell.closest('[role="row"]') as HTMLElement;
    expect(row).toHaveAccessibleDescription('matched invoice total');
    const alphaRow = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]') as HTMLElement;
    expect(alphaRow).not.toHaveAttribute('aria-describedby');
  });

  it('renders the empty state when there are no rows', () => {
    render(
      <Harness rows={[]} emptyState={<EmptyState title="No documents yet" description="Upload a file to start." />} />,
    );
    expect(screen.getByRole('heading', { name: 'No documents yet' })).toBeInTheDocument();
  });

  it('renders loading rows while the first page loads', () => {
    render(<Harness rows={[]} isLoading emptyState={<p>No documents yet</p>} />);
    expect(screen.getByRole('status', { name: 'Loading' })).toBeInTheDocument();
    expect(screen.queryByText('No documents yet')).not.toBeInTheDocument();
  });

  it('keeps rows visible and marks itself busy when reloading', () => {
    const { container } = render(<Harness isLoading />);
    expect(screen.getByRole('rowheader', { name: 'Alpha.pdf' })).toBeInTheDocument();
    expect(container.firstElementChild).toHaveAttribute('aria-busy', 'true');
  });

  it('uses the data face for numeric columns', () => {
    render(<Harness />);
    const alphaRow = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]') as HTMLElement;
    expect(within(alphaRow).getByRole('gridcell', { name: '3' })).toBeInTheDocument();
  });
});

describe('BoardTable on a narrow screen', () => {
  const PRIORITY_COLUMNS: BoardColumn<Doc>[] = [
    { id: 'name', label: 'Name', hideOnNarrow: true, render: (d) => d.name },
    { id: 'pages', label: 'Pages', align: 'end', width: 90, render: (d) => d.pages },
    { id: 'type', label: 'Type', width: '120px', hideOnNarrow: true, render: () => 'PDF' },
  ];

  const setNarrow = (narrow: boolean) => {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: createResponsiveMatchMediaMock({ 'max-width: 719px': narrow }),
    });
  };

  afterEach(() => setNarrow(false));

  it('drops low-priority columns but always keeps the row header', () => {
    setNarrow(true);
    render(<Harness columns={PRIORITY_COLUMNS} selectionMode="multiple" />);
    const headers = screen.getAllByRole('columnheader').map((h) => h.textContent);
    expect(headers).toContain('Name');
    expect(headers).toContain('Pages');
    expect(headers).not.toContain('Type');
    expect(screen.getByRole('rowheader', { name: 'Alpha.pdf' })).toBeInTheDocument();
  });

  it('stacks rows instead of widening the table', () => {
    setNarrow(true);
    const { container } = render(<Harness columns={PRIORITY_COLUMNS} selectionMode="multiple" />);
    expect(container.firstElementChild).toHaveAttribute('data-layout', 'stacked');
    expect(container.firstElementChild).toHaveAttribute('data-selectable', 'true');
    // No minimum width: nothing can push the page sideways.
    expect(screen.getByRole('grid').style.minWidth).toBe('');
  });

  /** The outermost element in `row` whose whole text is exactly `text`: one visible line. */
  const visibleLine = (row: HTMLElement, text: string) =>
    within(row).getAllByText((_, el) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim() === text)[0];

  it('folds the dropped columns into one unlabelled meta line, announced with labels', () => {
    setNarrow(true);
    const columns: BoardColumn<Doc>[] = [
      { id: 'name', label: 'Name', render: (d) => d.name },
      { id: 'pages', label: 'Pages', align: 'end', width: 90, hideOnNarrow: true, render: (d) => d.pages },
      { id: 'type', label: 'Type', width: 80, hideOnNarrow: true, render: () => 'PDF' },
    ];
    render(<Harness columns={columns} renderRowDetail={(d) => d.snippet} />);
    const alphaRow = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]') as HTMLElement;
    // The visible line reads exactly "3 · PDF": values only, no "Pages"/"Type" labels.
    expect(visibleLine(alphaRow, '3 · PDF')).toBeInTheDocument();
    expect(alphaRow).toHaveAccessibleDescription('Pages: 3; Type: PDF;');
    // The consumer's own detail is announced first; the folded fields follow it.
    const bravoRow = screen.getByRole('rowheader', { name: 'Bravo.pdf' }).closest('[role="row"]') as HTMLElement;
    expect(bravoRow).toHaveAccessibleDescription('matched invoice total Pages: 12; Type: PDF;');
    // Folded values stay out of the row header's name.
    expect(screen.getByRole('rowheader', { name: 'Alpha.pdf' })).toBeInTheDocument();
  });

  it('leads the second line with the tag and marks, and gives long text its own line', () => {
    setNarrow(true);
    const columns: BoardColumn<Doc>[] = [
      { id: 'name', label: 'Name', render: (d) => d.name },
      { id: 'state', label: 'State', width: 100, hideOnNarrow: true, fold: 'mark', render: () => 'FAILED' },
      { id: 'reason', label: 'Reason', hideOnNarrow: true, render: (d) => `Reason for ${d.name}` },
      { id: 'pages', label: 'Pages', width: 90, hideOnNarrow: true, render: (d) => d.pages, foldValue: (d) => (d.pages > 5 ? `${d.pages} p` : null) },
    ];
    render(<Harness columns={columns} renderRowTag={(d) => (d.isNew ? 'NEW' : null)} />);
    const bravoRow = screen.getByRole('rowheader', { name: 'Bravo.pdf' }).closest('[role="row"]') as HTMLElement;
    const line = visibleLine(bravoRow, 'NEWFAILED12 p');
    // A column without a width is long text: its own clamped line, not part of the meta.
    const text = within(bravoRow).getByText('Reason for Bravo.pdf');
    expect(line).not.toContainElement(text);
    expect(line).not.toHaveTextContent('Reason');
    // foldValue returning null leaves the value off the line but not out of the description.
    const alphaRow = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]') as HTMLElement;
    expect(visibleLine(alphaRow, 'FAILED')).toBeInTheDocument();
    expect(alphaRow).toHaveAccessibleDescription('State: FAILED; Reason: Reason for Alpha.pdf; Pages: 3;');
  });

  it('orders the meta line by foldOrder but announces fields in column order', () => {
    setNarrow(true);
    const columns: BoardColumn<Doc>[] = [
      { id: 'name', label: 'Name', render: (d) => d.name },
      { id: 'source', label: 'Source', width: 90, hideOnNarrow: true, foldOrder: 10, render: () => 'Upload' },
      { id: 'pages', label: 'Pages', width: 90, hideOnNarrow: true, render: (d) => d.pages },
    ];
    render(<Harness columns={columns} />);
    const alphaRow = screen.getByRole('rowheader', { name: 'Alpha.pdf' }).closest('[role="row"]') as HTMLElement;
    expect(visibleLine(alphaRow, '3 · Upload')).toBeInTheDocument();
    expect(alphaRow).toHaveAccessibleDescription('Source: Upload; Pages: 3;');
  });

  it('shows the row tag only on a narrow screen', () => {
    setNarrow(false);
    render(<Harness renderRowTag={() => 'NEW'} />);
    expect(screen.queryByText('NEW')).not.toBeInTheDocument();
  });

  it('keeps every column and sets no minimum width on a wide screen', () => {
    setNarrow(false);
    const { container } = render(<Harness columns={PRIORITY_COLUMNS} />);
    expect(screen.getByRole('columnheader', { name: 'Type' })).toBeInTheDocument();
    expect(screen.getByRole('grid').style.minWidth).toBe('');
    expect(container.firstElementChild).not.toHaveAttribute('data-layout');
  });
});

describe('BoardTable styling', () => {
  const css = readFileSync(resolve(__dirname, 'BoardTable.module.css'), 'utf8');
  it('tints selected rows and marks them with an accent edge', () => {
    expect(css).toMatch(/\.row\[data-selected\][^{]*\{[^}]*accent-soft/);
    expect(css).toMatch(/\.row\[data-selected\] > \.cell:first-child\s*\{[^}]*inset 3px 0 0 var\(--accent\)/);
  });
  it('clips to its rounded card without becoming a scroll container', () => {
    expect(css).toMatch(/\.container\s*\{[^}]*overflow:\s*clip/);
  });
});
