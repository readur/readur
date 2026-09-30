import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FilterChip } from './FilterChip';

describe('FilterChip', () => {
  it('renders the label and value inside one named button', () => {
    render(<FilterChip label="Type" value="PDF" isActive onPress={() => {}} />);
    const chip = screen.getByRole('button', { name: 'Type PDF' });
    expect(chip).toHaveAttribute('aria-pressed', 'true');
  });

  it('reports the inactive state', () => {
    render(<FilterChip label="Type" isActive={false} onPress={() => {}} />);
    expect(screen.getByRole('button', { name: 'Type' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('fires onPress from the keyboard', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(<FilterChip label="Type" isActive={false} onPress={onPress} />);
    await user.tab();
    await user.keyboard('{Enter}');
    await user.keyboard(' ');
    expect(onPress).toHaveBeenCalledTimes(2);
  });

  it('shows a labelled clear button only when active', async () => {
    const onClear = vi.fn();
    const onPress = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(<FilterChip label="Type" value="PDF" isActive onPress={onPress} onClear={onClear} />);
    await user.click(screen.getByRole('button', { name: 'Clear Type filter' }));
    expect(onClear).toHaveBeenCalledTimes(1);
    expect(onPress).not.toHaveBeenCalled();
    rerender(<FilterChip label="Type" isActive={false} onPress={onPress} onClear={onClear} />);
    expect(screen.queryByRole('button', { name: 'Clear Type filter' })).not.toBeInTheDocument();
  });

  it('opens popover content and clearing does not open it', async () => {
    const onClear = vi.fn();
    const user = userEvent.setup();
    render(
      <FilterChip label="Status" value="2" isActive onClear={onClear} popover={<p>Status options</p>} />,
    );
    await user.click(screen.getByRole('button', { name: 'Clear Status filter' }));
    expect(onClear).toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    const trigger = screen.getByRole('button', { name: 'Status 2' });
    await user.click(trigger);
    expect(await screen.findByRole('dialog', { name: 'Status' })).toHaveTextContent('Status options');
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});
