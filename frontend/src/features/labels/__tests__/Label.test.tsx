import { describe, test, expect, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Label from '../Label';
import { renderWithProviders } from '../../../test/test-utils';
import { createMockLabel, createMockSystemLabel } from '../../../test/label-test-utils';

const mockLabel = createMockLabel({ name: 'Test Label', color: '#ff0000', document_count: 5, source_count: 2 });
const systemLabel = createMockSystemLabel({ name: 'Important', color: '#d73a49' });

const renderLabel = (props: Partial<React.ComponentProps<typeof Label>> = {}) =>
  renderWithProviders(<Label label={mockLabel} {...props} />);

/** The tag's root element (it carries the label id). */
const tagOf = (name: string) => screen.getByText(name).closest('[data-label]') as HTMLElement;
const swatchOf = (name: string) => tagOf(name).querySelector('[data-swatch]') as HTMLElement;

describe('Label', () => {
  describe('rendering', () => {
    test('renders the label name', () => {
      renderLabel();
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('renders a colour swatch next to the name', () => {
      renderLabel();
      expect(swatchOf('Test Label')).toBeInTheDocument();
    });

    test('keeps the name visible, so colour is never the only cue', () => {
      renderLabel({ label: { ...mockLabel, color: '#ffffff' } });
      expect(screen.getByText('Test Label')).toBeVisible();
      expect(swatchOf('Test Label')).toHaveAttribute('aria-hidden', 'true');
    });

    test('renders filled by default', () => {
      renderLabel();
      expect(tagOf('Test Label')).toBeInTheDocument();
    });

    test('renders the outlined variant', () => {
      renderLabel({ variant: 'outlined' });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('is plain text (no button) when not clickable', () => {
      renderLabel();
      expect(screen.queryByRole('button')).not.toBeInTheDocument();
    });
  });

  describe('colour', () => {
    test('passes the label colour to the swatch', () => {
      renderLabel();
      expect(swatchOf('Test Label').style.getPropertyValue('--label-color')).toBe('#ff0000');
    });

    test('accepts short hex colours', () => {
      renderLabel({ label: { ...mockLabel, color: '#0f0' } });
      expect(swatchOf('Test Label').style.getPropertyValue('--label-color')).toBe('#0f0');
    });

    test('ignores a colour that is not a hex value', () => {
      renderLabel({ label: { ...mockLabel, color: 'red; background: url(x)' } });
      expect(swatchOf('Test Label').style.getPropertyValue('--label-color')).toBe('');
    });

    test('never paints the name on the label colour (dark colour)', () => {
      renderLabel({ label: { ...mockLabel, color: '#000000' } });
      expect(screen.getByText('Test Label').style.color).toBe('');
    });

    test('never paints the name on the label colour (light colour)', () => {
      renderLabel({ label: { ...mockLabel, color: '#ffffff' } });
      expect(screen.getByText('Test Label').style.backgroundColor).toBe('');
    });
  });

  describe('sizes', () => {
    test.each(['small', 'medium', 'large'] as const)('renders the %s size', (size) => {
      renderLabel({ size });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });
  });

  describe('document count', () => {
    test('shows the count when showCount is true', () => {
      renderLabel({ showCount: true });
      expect(screen.getByText('(5)')).toBeInTheDocument();
    });

    test('hides the count when showCount is false', () => {
      renderLabel({ showCount: false });
      expect(screen.queryByText('(5)')).not.toBeInTheDocument();
    });

    test('hides a zero count', () => {
      renderLabel({ label: { ...mockLabel, document_count: 0 }, showCount: true });
      expect(screen.queryByText('(0)')).not.toBeInTheDocument();
    });

    test('handles an undefined count', () => {
      renderLabel({ label: { ...mockLabel, document_count: undefined }, showCount: true });
      expect(screen.queryByText(/\(\d+\)/)).not.toBeInTheDocument();
    });
  });

  describe('clickable', () => {
    test('is a named button when onClick is given', () => {
      renderLabel({ onClick: vi.fn() });
      expect(screen.getByRole('button', { name: /test label/i })).toBeInTheDocument();
    });

    test('calls onClick with the label id', async () => {
      const onClick = vi.fn();
      renderLabel({ onClick });
      await userEvent.click(screen.getByRole('button', { name: /test label/i }));
      expect(onClick).toHaveBeenCalledWith(mockLabel.id);
    });

    test('is reachable with Tab and activates with Enter and Space', async () => {
      const onClick = vi.fn();
      const user = userEvent.setup();
      renderLabel({ onClick });
      await user.tab();
      expect(screen.getByRole('button', { name: /test label/i })).toHaveFocus();
      await user.keyboard('{Enter}');
      await user.keyboard(' ');
      expect(onClick).toHaveBeenCalledTimes(2);
    });

    test('does not call onClick when disabled', async () => {
      const onClick = vi.fn();
      renderLabel({ onClick, disabled: true });
      const button = screen.getByRole('button', { name: /test label/i });
      expect(button).toBeDisabled();
      await userEvent.click(button);
      expect(onClick).not.toHaveBeenCalled();
    });

    test('marks the tag as disabled', () => {
      renderLabel({ disabled: true });
      expect(tagOf('Test Label')).toHaveAttribute('data-disabled', 'true');
    });
  });

  describe('deletion', () => {
    test('shows a named remove button when deletable', () => {
      renderLabel({ deletable: true, onDelete: vi.fn() });
      expect(screen.getByRole('button', { name: 'Remove Test Label' })).toBeInTheDocument();
    });

    test('has no remove button when not deletable', () => {
      renderLabel({ onDelete: vi.fn() });
      expect(screen.queryByRole('button', { name: /remove/i })).not.toBeInTheDocument();
    });

    test('calls onDelete with the label id', async () => {
      const onDelete = vi.fn();
      renderLabel({ deletable: true, onDelete });
      await userEvent.click(screen.getByRole('button', { name: 'Remove Test Label' }));
      expect(onDelete).toHaveBeenCalledWith(mockLabel.id);
    });

    test('does not delete when disabled', async () => {
      const onDelete = vi.fn();
      renderLabel({ deletable: true, onDelete, disabled: true });
      await userEvent.click(screen.getByRole('button', { name: 'Remove Test Label' }));
      expect(onDelete).not.toHaveBeenCalled();
    });

    test('removing does not also click the tag', async () => {
      const onClick = vi.fn();
      const onDelete = vi.fn();
      renderLabel({ onClick, deletable: true, onDelete });
      await userEvent.click(screen.getByRole('button', { name: 'Remove Test Label' }));
      expect(onDelete).toHaveBeenCalledWith(mockLabel.id);
      expect(onClick).not.toHaveBeenCalled();
    });

    test('removes with the keyboard', async () => {
      const onDelete = vi.fn();
      const user = userEvent.setup();
      renderLabel({ deletable: true, onDelete });
      await user.tab();
      expect(screen.getByRole('button', { name: 'Remove Test Label' })).toHaveFocus();
      await user.keyboard('{Enter}');
      expect(onDelete).toHaveBeenCalledTimes(1);
    });
  });

  describe('system labels', () => {
    test('renders a system label', () => {
      renderLabel({ label: systemLabel });
      expect(screen.getByText('Important')).toBeInTheDocument();
    });

    test('never offers to remove a system label', () => {
      renderLabel({ label: systemLabel, deletable: true, onDelete: vi.fn() });
      expect(screen.queryByRole('button', { name: /remove/i })).not.toBeInTheDocument();
    });
  });

  describe('icons', () => {
    test('renders a known icon as decoration', () => {
      renderLabel({ label: { ...mockLabel, icon: 'star' } });
      expect(screen.getByTestId('StarIcon').closest('[aria-hidden="true"]')).not.toBeNull();
    });

    test('renders the work icon', () => {
      renderLabel({ label: { ...mockLabel, icon: 'work' } });
      expect(screen.getByTestId('WorkIcon')).toBeInTheDocument();
    });

    test('renders without an icon when none is set', () => {
      renderLabel({ label: { ...mockLabel, icon: undefined } });
      expect(screen.queryByTestId('StarIcon')).not.toBeInTheDocument();
    });

    test('ignores an unknown icon', () => {
      renderLabel({ label: { ...mockLabel, icon: 'unknown_icon' } });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });
  });

  describe('edge cases', () => {
    test('applies a custom className', () => {
      renderLabel({ className: 'custom-label-class' });
      expect(tagOf('Test Label')).toHaveClass('custom-label-class');
    });

    test('renders very long names in full for assistive tech', () => {
      const name = 'This is a very long label name that might cause layout issues';
      renderLabel({ label: { ...mockLabel, name } });
      expect(screen.getByText(name)).toBeInTheDocument();
    });

    test('renders special characters literally', () => {
      const name = 'Label & Special <Characters> "Quotes"';
      renderLabel({ label: { ...mockLabel, name } });
      expect(screen.getByText(name)).toBeInTheDocument();
    });
  });
});
