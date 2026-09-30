import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LabelSelector from '../LabelSelector';
import type { LabelData } from '../labelData';
import { renderWithProviders } from '../../../test/test-utils';
import { testDataBuilders } from '../../../test/label-test-utils';

const mockLabels = testDataBuilders.createTypicalLabelSet();
const [important, work, , projectAlpha] = mockLabels;

const newLabel: LabelData = {
  id: 'new-label',
  name: 'New Label',
  color: '#0969da',
  is_system: false,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  document_count: 0,
  source_count: 0,
};

const renderSelector = (props: Partial<React.ComponentProps<typeof LabelSelector>> = {}) =>
  renderWithProviders(
    <LabelSelector selectedLabels={[]} availableLabels={mockLabels} onLabelsChange={vi.fn()} {...props} />,
  );

const optionNames = () => screen.getAllByRole('option').map((o) => o.textContent ?? '');
const openList = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole('combobox'));
  return screen.findByRole('listbox');
};

describe('LabelSelector', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
  });

  describe('rendering', () => {
    test('renders a named combobox', () => {
      renderSelector();
      expect(screen.getByRole('combobox', { name: 'Labels' })).toBeInTheDocument();
    });

    test('uses a visible label when given one', () => {
      renderSelector({ label: 'Document labels' });
      expect(screen.getByRole('combobox', { name: 'Document labels' })).toBeInTheDocument();
    });

    test('shows the default placeholder', () => {
      renderSelector();
      expect(screen.getByPlaceholderText('Search or create labels...')).toBeInTheDocument();
    });

    test('shows a custom placeholder', () => {
      renderSelector({ placeholder: 'Custom placeholder' });
      expect(screen.getByPlaceholderText('Custom placeholder')).toBeInTheDocument();
    });

    test('lists the selected labels as tags', () => {
      renderSelector({ selectedLabels: [important, projectAlpha] });
      const list = screen.getByRole('list', { name: 'Selected labels' });
      expect(within(list).getByText('Important')).toBeInTheDocument();
      expect(within(list).getByText('Project Alpha')).toBeInTheDocument();
    });

    test('drops the placeholder once something is selected', () => {
      renderSelector({ selectedLabels: [important] });
      expect(screen.queryByPlaceholderText('Search or create labels...')).not.toBeInTheDocument();
    });
  });

  describe('selection', () => {
    test('adds a picked label', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ onLabelsChange });
      await openList(user);
      await user.click(screen.getByRole('option', { name: /important/i }));
      expect(onLabelsChange).toHaveBeenCalledWith([important]);
    });

    test('leaves already selected labels out of the list', async () => {
      renderSelector({ selectedLabels: [important] });
      await openList(user);
      expect(optionNames().some((n) => n.includes('Important'))).toBe(false);
      expect(optionNames().some((n) => n.includes('Work'))).toBe(true);
    });

    test('replaces the choice in single mode', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ onLabelsChange, multiple: false, selectedLabels: [work] });
      await openList(user);
      await user.click(screen.getByRole('option', { name: /important/i }));
      expect(onLabelsChange).toHaveBeenCalledWith([important]);
    });

    test('appends in multiple mode', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ onLabelsChange, selectedLabels: [important] });
      await openList(user);
      await user.click(screen.getByRole('option', { name: /^work/i }));
      expect(onLabelsChange).toHaveBeenCalledWith([important, work]);
    });

    test('clears the search text after picking', async () => {
      renderSelector();
      await user.type(screen.getByRole('combobox'), 'imp');
      await user.click(await screen.findByRole('option', { name: /important/i }));
      expect(screen.getByRole('combobox')).toHaveValue('');
    });
  });

  describe('removal', () => {
    test('removes a tag with its remove button', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ selectedLabels: [projectAlpha], onLabelsChange });
      await user.click(screen.getByRole('button', { name: 'Remove Project Alpha' }));
      expect(onLabelsChange).toHaveBeenCalledWith([]);
    });

    test('keeps the other tags when one is removed', async () => {
      const onLabelsChange = vi.fn();
      const invoices = mockLabels[4];
      renderSelector({ selectedLabels: [projectAlpha, invoices], onLabelsChange });
      await user.click(screen.getByRole('button', { name: 'Remove Project Alpha' }));
      expect(onLabelsChange).toHaveBeenCalledWith([invoices]);
    });

    test('offers no remove buttons when disabled', () => {
      renderSelector({ selectedLabels: [projectAlpha], disabled: true });
      expect(screen.queryByRole('button', { name: /remove/i })).not.toBeInTheDocument();
    });
  });

  describe('grouping', () => {
    test('groups system labels and the user’s own labels', async () => {
      renderSelector();
      const list = await openList(user);
      expect(within(list).getByText('System Labels')).toBeInTheDocument();
      expect(within(list).getByText('My Labels')).toBeInTheDocument();
    });

    test('shows only the system group when there are no user labels', async () => {
      renderSelector({ availableLabels: mockLabels.filter((l) => l.is_system) });
      const list = await openList(user);
      expect(within(list).getByText('System Labels')).toBeInTheDocument();
      expect(within(list).queryByText('My Labels')).not.toBeInTheDocument();
      expect(within(list).queryByText('Project Alpha')).not.toBeInTheDocument();
    });
  });

  describe('search', () => {
    test('filters by name', async () => {
      renderSelector();
      await user.type(screen.getByRole('combobox'), 'work');
      await screen.findByRole('listbox');
      expect(optionNames()).toHaveLength(1);
      expect(optionNames()[0]).toContain('Work');
    });

    test('filters by description as well', async () => {
      renderSelector();
      await user.type(screen.getByRole('combobox'), 'priority');
      expect(await screen.findByRole('option', { name: /important/i })).toBeInTheDocument();
    });

    test('says so when nothing matches', async () => {
      renderSelector();
      await user.type(screen.getByRole('combobox'), 'nonexistent');
      expect(await screen.findByText('No labels match "nonexistent"')).toBeInTheDocument();
    });

    test('says so when there are no labels at all', async () => {
      renderSelector({ availableLabels: [] });
      await openList(user);
      expect(screen.getByText('No labels available')).toBeInTheDocument();
    });
  });

  describe('creating a label', () => {
    test('offers to create a label for new text', async () => {
      renderSelector({ onCreateLabel: vi.fn() });
      await user.type(screen.getByRole('combobox'), 'New Label');
      expect(await screen.findByRole('option', { name: 'Create label "New Label"' })).toBeInTheDocument();
    });

    test('does not offer create without onCreateLabel', async () => {
      renderSelector();
      await user.type(screen.getByRole('combobox'), 'New Label');
      await screen.findByRole('listbox');
      expect(screen.queryByRole('option', { name: /create label/i })).not.toBeInTheDocument();
    });

    test('does not offer create when showCreateButton is false', async () => {
      renderSelector({ onCreateLabel: vi.fn(), showCreateButton: false });
      await user.type(screen.getByRole('combobox'), 'New Label');
      await screen.findByRole('listbox');
      expect(screen.queryByRole('option', { name: /create label/i })).not.toBeInTheDocument();
    });

    test('does not offer create for an existing name (any case)', async () => {
      renderSelector({ onCreateLabel: vi.fn() });
      await user.type(screen.getByRole('combobox'), 'important');
      await screen.findByRole('listbox');
      expect(screen.queryByRole('option', { name: /create label/i })).not.toBeInTheDocument();
    });

    test('opens the dialog prefilled and adds the created label', async () => {
      const onCreateLabel = vi.fn().mockResolvedValue(newLabel);
      const onLabelsChange = vi.fn();
      renderSelector({ onCreateLabel, onLabelsChange, selectedLabels: [important] });
      await user.type(screen.getByRole('combobox'), 'New Label');
      await user.click(await screen.findByRole('option', { name: 'Create label "New Label"' }));

      const dialog = await screen.findByRole('dialog', { name: 'Create New Label' });
      expect(within(dialog).getByRole('textbox', { name: /label name/i })).toHaveValue('New Label');
      await user.click(within(dialog).getByRole('button', { name: 'Create' }));

      await waitFor(() =>
        expect(onCreateLabel).toHaveBeenCalledWith({
          name: 'New Label',
          description: undefined,
          color: '#0969da',
          background_color: undefined,
          icon: undefined,
        }),
      );
      expect(onLabelsChange).toHaveBeenCalledWith([important, newLabel]);
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    });

    test('keeps the dialog open with the error when creating fails', async () => {
      const onCreateLabel = vi.fn().mockRejectedValue(new Error('Create failed'));
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
      renderSelector({ onCreateLabel });
      await user.type(screen.getByRole('combobox'), 'New Label');
      await user.click(await screen.findByRole('option', { name: 'Create label "New Label"' }));
      const dialog = await screen.findByRole('dialog', { name: 'Create New Label' });
      await user.click(within(dialog).getByRole('button', { name: 'Create' }));

      expect(await within(dialog).findByText('Create failed')).toBeInTheDocument();
      expect(consoleError).toHaveBeenCalledWith('Failed to create label:', expect.any(Error));
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      consoleError.mockRestore();
    });
  });

  describe('max tags', () => {
    test('ignores picks beyond the limit', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ selectedLabels: [important, work], onLabelsChange, maxTags: 2 });
      await openList(user);
      await user.click(screen.getByRole('option', { name: /project alpha/i }));
      expect(onLabelsChange).not.toHaveBeenCalled();
    });

    test('allows picks under the limit', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ selectedLabels: [important], onLabelsChange, maxTags: 2 });
      await openList(user);
      await user.click(screen.getByRole('option', { name: /^work/i }));
      expect(onLabelsChange).toHaveBeenCalledWith([important, work]);
    });
  });

  describe('disabled', () => {
    test('disables the field', () => {
      renderSelector({ disabled: true });
      expect(screen.getByRole('combobox')).toBeDisabled();
    });

    test('cannot create while disabled', () => {
      renderSelector({ disabled: true, onCreateLabel: vi.fn() });
      expect(screen.getByRole('combobox')).toBeDisabled();
      expect(screen.queryByRole('option', { name: /create label/i })).not.toBeInTheDocument();
    });
  });

  describe('sizes', () => {
    test('renders the small size', () => {
      renderSelector({ size: 'small' });
      expect(screen.getByRole('combobox').closest('[data-size]')).toHaveAttribute('data-size', 'small');
    });

    test('renders the medium size by default', () => {
      renderSelector();
      expect(screen.getByRole('combobox').closest('[data-size]')).toHaveAttribute('data-size', 'medium');
    });
  });

  describe('keyboard', () => {
    test('opens with ArrowDown and picks with Enter', async () => {
      const onLabelsChange = vi.fn();
      renderSelector({ onLabelsChange });
      await user.tab();
      expect(screen.getByRole('combobox')).toHaveFocus();
      await user.keyboard('{ArrowDown}');
      await screen.findByRole('listbox');
      await user.keyboard('{Enter}');
      expect(onLabelsChange).toHaveBeenCalledTimes(1);
    });

    test('closes the list with Escape', async () => {
      renderSelector();
      await openList(user);
      await user.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument());
    });
  });
});
