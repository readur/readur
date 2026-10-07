import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { LabelChip } from './Label';

describe('LabelChip', () => {
  it('shows a colour dot and the name', () => {
    const { container } = render(<LabelChip name="Taxes" color="#C0841A" dataId="l1" />);
    expect(screen.getByText('Taxes')).toBeInTheDocument();
    const root = container.querySelector('[data-label="l1"]') as HTMLElement;
    expect(root.style.getPropertyValue('--label-color')).toBe('#C0841A');
    expect(container.querySelector('[data-swatch]')).toHaveAttribute('aria-hidden', 'true');
  });

  it('falls back to a neutral dot without a colour', () => {
    const { container } = render(<LabelChip name="Inbox" dataId="l2" />);
    expect((container.querySelector('[data-label="l2"]') as HTMLElement).style.getPropertyValue('--label-color')).toBe('');
    expect(container.querySelector('[data-swatch]')).not.toBeNull();
  });

  it('shows a count only when above zero', () => {
    const { rerender } = render(<LabelChip name="Taxes" count={3} />);
    expect(screen.getByText('(3)')).toBeInTheDocument();
    rerender(<LabelChip name="Taxes" count={0} />);
    expect(screen.queryByText('(0)')).not.toBeInTheDocument();
  });

  it('is a button when pressable', async () => {
    const onPress = vi.fn();
    render(<LabelChip name="Taxes" onPress={onPress} />);
    await userEvent.click(screen.getByRole('button', { name: 'Taxes' }));
    expect(onPress).toHaveBeenCalled();
  });

  it('removes with a named button', async () => {
    const onRemove = vi.fn();
    render(<LabelChip name="Taxes" onRemove={onRemove} removeLabel="Remove Taxes" />);
    await userEvent.click(screen.getByRole('button', { name: 'Remove Taxes' }));
    expect(onRemove).toHaveBeenCalled();
  });
});
