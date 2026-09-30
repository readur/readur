import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Checkbox } from './Checkbox';

describe('Checkbox', () => {
  it('renders with an accessible name', () => {
    render(<Checkbox label="Select all" />);
    expect(screen.getByRole('checkbox', { name: 'Select all' })).not.toBeChecked();
  });

  it('toggles with Space', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Checkbox label="Select all" onChange={onChange} />);
    await user.tab();
    await user.keyboard(' ');
    expect(onChange).toHaveBeenCalledWith(true);
    expect(screen.getByRole('checkbox', { name: 'Select all' })).toBeChecked();
  });

  it('reports the indeterminate state', () => {
    render(<Checkbox label="Some" isIndeterminate />);
    expect(screen.getByRole('checkbox', { name: 'Some' })).toBePartiallyChecked();
  });

  it('supports aria-label without visible text, and disabled', () => {
    render(<Checkbox aria-label="Select row" isDisabled />);
    expect(screen.getByRole('checkbox', { name: 'Select row' })).toBeDisabled();
  });
});
