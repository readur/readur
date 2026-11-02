import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { ComboBox, ComboBoxItem } from './ComboBox';

const setup = () =>
  render(
    <ComboBox label="Label">
      <ComboBoxItem id="inv">Invoice</ComboBoxItem>
      <ComboBoxItem id="rec">Receipt</ComboBoxItem>
    </ComboBox>,
  );

describe('ComboBox', () => {
  it('renders a combobox with an accessible name and a labelled options button', () => {
    setup();
    expect(screen.getByRole('combobox', { name: 'Label' })).toBeInTheDocument();
    expect(screen.getByRole('button')).toHaveAccessibleName('Show options Label');
  });

  it('opens with ArrowDown, picks with Enter, closes with Escape', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('combobox', { name: 'Label' }));
    await user.keyboard('{ArrowDown}');
    expect(await screen.findByRole('listbox')).toBeInTheDocument();
    await user.keyboard('{ArrowDown}{Enter}');
    expect(screen.getByRole('combobox', { name: 'Label' })).not.toHaveValue('');
    await user.keyboard('{ArrowDown}{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('filters options as the user types', async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole('combobox', { name: 'Label' }), 'rec');
    expect(await screen.findByRole('option', { name: 'Receipt' })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Invoice' })).not.toBeInTheDocument();
  });
});
