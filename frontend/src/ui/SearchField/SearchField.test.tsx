import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SearchField } from './SearchField';

describe('SearchField', () => {
  it('renders with an accessible name', () => {
    render(<SearchField label="Search documents" />);
    expect(screen.getByRole('searchbox', { name: 'Search documents' })).toBeInTheDocument();
  });

  it('clears with the clear button', async () => {
    const user = userEvent.setup();
    render(<SearchField label="Search" defaultValue="invoice" />);
    await user.click(screen.getByRole('button', { name: 'Clear' }));
    expect(screen.getByRole('searchbox', { name: 'Search' })).toHaveValue('');
  });

  it('clears with Escape', async () => {
    const onClear = vi.fn();
    const user = userEvent.setup();
    render(<SearchField label="Search" defaultValue="invoice" onClear={onClear} />);
    await user.click(screen.getByRole('searchbox', { name: 'Search' }));
    await user.keyboard('{Escape}');
    expect(screen.getByRole('searchbox', { name: 'Search' })).toHaveValue('');
    expect(onClear).toHaveBeenCalled();
  });
});
