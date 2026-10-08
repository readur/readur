import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Select, SelectItem } from './Select';

const renderSelect = (onSelectionChange = vi.fn()) =>
  render(
    <Select label="Status" placeholder="Pick one" onSelectionChange={onSelectionChange}>
      <SelectItem id="a">Alpha</SelectItem>
      <SelectItem id="b">Beta</SelectItem>
    </Select>,
  );

describe('Select', () => {
  it('renders a button named by its label', () => {
    renderSelect();
    expect(screen.getByRole('button', { name: /Status/ })).toBeInTheDocument();
  });

  it('opens with Enter, navigates with arrows and selects', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    renderSelect(onChange);
    await user.tab();
    await user.keyboard('{Enter}');
    expect(await screen.findByRole('listbox')).toBeInTheDocument();
    await user.keyboard('{ArrowDown}{Enter}');
    expect(onChange).toHaveBeenCalled();
  });

  it('closes with Escape', async () => {
    const user = userEvent.setup();
    renderSelect();
    await user.click(screen.getByRole('button', { name: /Status/ }));
    expect(await screen.findByRole('listbox')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('marks the chosen option as selected when reopened', async () => {
    const user = userEvent.setup();
    renderSelect();
    await user.click(screen.getByRole('button', { name: /Status/ }));
    await user.click(await screen.findByRole('option', { name: 'Beta' }));
    await user.click(screen.getByRole('button', { name: /Status/ }));
    expect(await screen.findByRole('option', { name: 'Beta' })).toHaveAttribute('data-selected', 'true');
  });
});
