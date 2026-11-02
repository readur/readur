import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Pagination, pageItems } from './Pagination';

describe('Pagination', () => {
  it('shows the x-y of N readout in a named nav', () => {
    render(<Pagination page={2} pageSize={25} total={60} onChange={vi.fn()} />);
    expect(screen.getByRole('navigation', { name: 'Pagination' })).toBeInTheDocument();
    expect(screen.getByText('26–50 of 60')).toBeInTheDocument();
  });

  it('shows 0–0 of 0 when empty and disables both buttons', () => {
    render(<Pagination page={1} pageSize={25} total={0} onChange={vi.fn()} />);
    expect(screen.getByText('0–0 of 0')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  it('pages with keyboard-activated buttons', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Pagination page={2} pageSize={25} total={100} onChange={onChange} />);
    await user.tab();
    await user.keyboard('{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(1, 25);
    // Page-number buttons sit between Previous and Next in the tab order.
    screen.getByRole('button', { name: 'Next page' }).focus();
    await user.keyboard('{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(3, 25);
  });

  it('disables previous on first page and next on last page', () => {
    const { rerender } = render(<Pagination page={1} pageSize={10} total={30} onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
    rerender(<Pagination page={3} pageSize={10} total={30} onChange={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  it('changes page size through the select and resets to page 1', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Pagination page={3} pageSize={10} total={100} onChange={onChange} pageSizeOptions={[10, 25, 50]} />);
    await user.click(screen.getByRole('button', { name: /Rows per page/ }));
    await user.click(await screen.findByRole('option', { name: '25' }));
    expect(onChange).toHaveBeenCalledWith(1, 25);
  });

  it('compacts long page lists', () => {
    expect(pageItems(1, 3)).toEqual([1, 2, 3]);
    expect(pageItems(2, 26)).toEqual([1, 2, 3, 'gap', 26]);
    expect(pageItems(13, 26)).toEqual([1, 'gap', 12, 13, 14, 'gap', 26]);
    expect(pageItems(26, 26)).toEqual([1, 'gap', 25, 26]);
  });

  it('marks the current page and jumps to a page', async () => {
    const onChange = vi.fn();
    render(<Pagination page={2} pageSize={25} total={60} onChange={onChange} />);
    expect(screen.getByRole('button', { name: 'Page 2' })).toHaveAttribute('aria-current', 'page');
    await userEvent.click(screen.getByRole('button', { name: 'Page 3' }));
    expect(onChange).toHaveBeenCalledWith(3, 25);
  });
});
