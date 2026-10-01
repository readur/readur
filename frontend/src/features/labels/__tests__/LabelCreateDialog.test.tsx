import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LabelCreateDialog from '../LabelCreateDialog';
import { renderWithProviders } from '../../../test/test-utils';
import { createMockLabel } from '../../../test/label-test-utils';

const editingLabel = createMockLabel({
  name: 'Existing Label',
  description: 'An existing label',
  color: '#ff0000',
  icon: 'star',
  document_count: 5,
  source_count: 2,
});

const renderDialog = (props: Partial<React.ComponentProps<typeof LabelCreateDialog>> = {}) =>
  renderWithProviders(<LabelCreateDialog open onClose={vi.fn()} onSubmit={vi.fn()} {...props} />);

const nameField = () => screen.getByRole('textbox', { name: /^name/i });
const descriptionField = () => screen.getByRole('textbox', { name: /description/i });
const colorField = () => screen.getByRole('textbox', { name: /custom color/i });
const submitButton = (name: RegExp | string = 'Create') => screen.getByRole('button', { name });
const colorGroup = () => screen.getByRole('radiogroup', { name: 'Color' });
const iconGroup = () => screen.getByRole('radiogroup', { name: 'Icon (optional)' });

/** Resolves only when the test says so, to hold the dialog in its saving state. */
function pendingSubmit() {
  let resolve!: () => void;
  const onSubmit = vi.fn().mockImplementation(() => new Promise<void>((r) => (resolve = r)));
  return { onSubmit, resolve: () => resolve() };
}

describe('LabelCreateDialog', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
  });

  describe('create mode', () => {
    test('is a dialog named "Create a label"', () => {
      renderDialog();
      expect(screen.getByRole('dialog', { name: 'Create a label' })).toBeInTheDocument();
    });

    test('renders every field', () => {
      renderDialog();
      expect(nameField()).toBeInTheDocument();
      expect(descriptionField()).toBeInTheDocument();
      expect(colorField()).toBeInTheDocument();
      expect(colorGroup()).toBeInTheDocument();
      expect(iconGroup()).toBeInTheDocument();
      expect(screen.getByRole('group', { name: 'Preview' })).toBeInTheDocument();
    });

    test('prefills the name', () => {
      renderDialog({ prefilledName: 'Prefilled Name' });
      expect(nameField()).toHaveValue('Prefilled Name');
    });

    test('starts with the default colour', () => {
      renderDialog();
      expect(colorField()).toHaveValue('#0969da');
      expect(within(colorGroup()).getByRole('radio', { name: 'Blue' })).toBeChecked();
    });

    test('shows a Create button', () => {
      renderDialog();
      expect(submitButton()).toBeInTheDocument();
    });

    test('focuses the name field when it opens', () => {
      renderDialog();
      expect(nameField()).toHaveFocus();
    });

    test('renders nothing while closed', () => {
      renderDialog({ open: false });
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  describe('edit mode', () => {
    test('is named "Edit Label"', () => {
      renderDialog({ editingLabel });
      expect(screen.getByRole('dialog', { name: 'Edit label' })).toBeInTheDocument();
    });

    test('fills the form from the label', () => {
      renderDialog({ editingLabel });
      expect(nameField()).toHaveValue('Existing Label');
      expect(descriptionField()).toHaveValue('An existing label');
      expect(colorField()).toHaveValue('#ff0000');
      expect(within(iconGroup()).getByRole('radio', { name: 'Star' })).toBeChecked();
    });

    test('shows an Update button', () => {
      renderDialog({ editingLabel });
      expect(submitButton('Update')).toBeInTheDocument();
    });
  });

  describe('validation', () => {
    test('disables submit while the name is empty', () => {
      renderDialog();
      expect(submitButton()).toBeDisabled();
    });

    test('enables submit once a name is typed', async () => {
      renderDialog();
      await user.type(nameField(), 'Test Label');
      expect(submitButton()).toBeEnabled();
    });

    test('keeps submit disabled for a name of only spaces', async () => {
      renderDialog();
      await user.type(nameField(), '   ');
      expect(submitButton()).toBeDisabled();
    });

    test('rejects commas in the name', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.type(nameField(), 'a,b');
      await user.click(submitButton());
      expect(screen.getByText('Label names cannot contain commas.')).toBeInTheDocument();
      expect(nameField()).toHaveAttribute('aria-invalid', 'true');
      expect(onSubmit).not.toHaveBeenCalled();
    });

    test('rejects an encoded comma in the name', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.type(nameField(), 'a%2Cb');
      await user.click(submitButton());
      expect(onSubmit).not.toHaveBeenCalled();
    });

    test('clears the name error when the name changes', async () => {
      renderDialog();
      await user.type(nameField(), 'a,b');
      await user.click(submitButton());
      await user.type(nameField(), 'c');
      expect(screen.queryByText('Label names cannot contain commas.')).not.toBeInTheDocument();
    });

    test('flags a colour that is not hex and blocks submit', async () => {
      renderDialog({ prefilledName: 'Name' });
      await user.clear(colorField());
      await user.type(colorField(), 'red');
      expect(colorField()).toHaveAttribute('aria-invalid', 'true');
      expect(submitButton()).toBeDisabled();
    });
  });

  describe('colour', () => {
    test('offers ten named preset colours', () => {
      renderDialog();
      expect(within(colorGroup()).getAllByRole('radio')).toHaveLength(10);
      expect(within(colorGroup()).getByRole('radio', { name: 'Red' })).toBeInTheDocument();
    });

    test('picks a preset colour', async () => {
      renderDialog();
      await user.click(within(colorGroup()).getByRole('radio', { name: 'Red' }));
      expect(colorField()).toHaveValue('#d73a49');
    });

    test('accepts a custom hex colour', async () => {
      renderDialog();
      await user.clear(colorField());
      await user.type(colorField(), '#abcdef');
      expect(colorField()).toHaveValue('#abcdef');
      expect(within(colorGroup()).queryByRole('radio', { checked: true })).not.toBeInTheDocument();
    });

    test('moves between presets with the arrow keys', async () => {
      renderDialog();
      const blue = within(colorGroup()).getByRole('radio', { name: 'Blue' });
      act(() => blue.focus());
      expect(blue).toHaveFocus();
      await user.keyboard('{ArrowRight}');
      expect(colorField()).toHaveValue('#d73a49');
    });
  });

  describe('icon', () => {
    test('offers None plus the named icons', () => {
      renderDialog();
      const radios = within(iconGroup()).getAllByRole('radio');
      expect(radios.length).toBeGreaterThan(5);
      expect(within(iconGroup()).getByRole('radio', { name: 'None' })).toBeInTheDocument();
    });

    test('selects None by default', () => {
      renderDialog();
      expect(within(iconGroup()).getByRole('radio', { name: 'None' })).toBeChecked();
    });

    test('selects an icon', async () => {
      renderDialog();
      await user.click(within(iconGroup()).getByRole('radio', { name: 'Star' }));
      expect(within(iconGroup()).getByRole('radio', { name: 'Star' })).toBeChecked();
    });

    test('goes back to no icon with None', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit, prefilledName: 'Name' });
      await user.click(within(iconGroup()).getByRole('radio', { name: 'Star' }));
      await user.click(within(iconGroup()).getByRole('radio', { name: 'None' }));
      expect(within(iconGroup()).getByRole('radio', { name: 'None' })).toBeChecked();
      await user.click(submitButton());
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ icon: undefined }));
    });
  });

  describe('preview', () => {
    const preview = () => screen.getByRole('group', { name: 'Preview' });

    test('shows filled and outlined previews of the name', () => {
      renderDialog({ prefilledName: 'Test Label' });
      expect(within(preview()).getAllByText('Test Label')).toHaveLength(2);
    });

    test('follows the name as it is typed', async () => {
      renderDialog();
      await user.type(nameField(), 'Dynamic Preview');
      expect(within(preview()).getAllByText('Dynamic Preview')).toHaveLength(2);
    });

    test('shows a placeholder name while empty', () => {
      renderDialog();
      expect(within(preview()).getAllByText('Label preview')).toHaveLength(2);
    });
  });

  describe('submitting', () => {
    test('sends the entered values', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.click(nameField());
      await user.paste('Test Label');
      await user.click(descriptionField());
      await user.paste('Test description');
      await user.click(submitButton());
      expect(onSubmit).toHaveBeenCalledWith({
        name: 'Test Label',
        description: 'Test description',
        color: '#0969da',
        background_color: undefined,
        icon: undefined,
      });
    });

    test('sends the edited values', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit, editingLabel });
      await user.clear(nameField());
      await user.paste('Updated Label');
      await user.click(submitButton('Update'));
      expect(onSubmit).toHaveBeenCalledWith({
        name: 'Updated Label',
        description: 'An existing label',
        color: '#ff0000',
        background_color: undefined,
        icon: 'star',
      });
    });

    test('sends only the name when nothing else is set', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.click(nameField());
      await user.paste('Minimal Label');
      await user.click(submitButton());
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ name: 'Minimal Label', description: undefined }));
    });

    test('trims the name', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.type(nameField(), '  Trimmed Label  ');
      await user.click(submitButton());
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ name: 'Trimmed Label' }));
    });

    test('closes after a successful save', async () => {
      const onClose = vi.fn();
      renderDialog({ onClose, onSubmit: vi.fn().mockResolvedValue(undefined) });
      await user.type(nameField(), 'Saved');
      await user.click(submitButton());
      await waitFor(() => expect(onClose).toHaveBeenCalled());
    });

    test('shows the server error and stays open', async () => {
      const onClose = vi.fn();
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
      renderDialog({ onClose, onSubmit: vi.fn().mockRejectedValue(new Error('Name already taken')) });
      await user.type(nameField(), 'Dup');
      await user.click(submitButton());
      expect(await screen.findByText('Name already taken')).toBeInTheDocument();
      expect(onClose).not.toHaveBeenCalled();
      consoleError.mockRestore();
    });

    test('submits with Enter from the name field', async () => {
      const onSubmit = vi.fn();
      renderDialog({ onSubmit });
      await user.type(nameField(), 'Test Label');
      await user.keyboard('{Enter}');
      expect(onSubmit).toHaveBeenCalled();
    });
  });

  describe('saving state', () => {
    test('marks the submit button busy while saving', async () => {
      const { onSubmit, resolve } = pendingSubmit();
      renderDialog({ onSubmit });
      await user.type(nameField(), 'Test Label');
      await user.click(submitButton());
      await waitFor(() => expect(screen.getByRole('button', { name: /saving/i })).toHaveAttribute('aria-busy', 'true'));
      resolve();
      await waitFor(() => expect(screen.queryByRole('button', { name: /saving/i })).not.toBeInTheDocument());
    });

    test('disables the fields while saving', async () => {
      const { onSubmit, resolve } = pendingSubmit();
      renderDialog({ onSubmit });
      await user.type(nameField(), 'Test Label');
      await user.click(submitButton());
      await waitFor(() => expect(nameField()).toBeDisabled());
      expect(descriptionField()).toBeDisabled();
      resolve();
      await waitFor(() => expect(nameField()).toBeEnabled());
    });
  });

  describe('closing', () => {
    test('Cancel calls onClose', async () => {
      const onClose = vi.fn();
      renderDialog({ onClose });
      await user.click(screen.getByRole('button', { name: 'Cancel' }));
      expect(onClose).toHaveBeenCalled();
    });

    test('Escape calls onClose', async () => {
      const onClose = vi.fn();
      renderDialog({ onClose });
      await user.keyboard('{Escape}');
      expect(onClose).toHaveBeenCalled();
    });

    test('cannot be closed while saving', async () => {
      const onClose = vi.fn();
      const { onSubmit, resolve } = pendingSubmit();
      renderDialog({ onClose, onSubmit });
      await user.type(nameField(), 'Test Label');
      void user.click(submitButton());
      await waitFor(() => expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled());
      await user.keyboard('{Escape}');
      expect(onClose).not.toHaveBeenCalled();
      resolve();
      await waitFor(() => expect(screen.queryByRole('button', { name: /saving/i })).not.toBeInTheDocument());
    });

    test('resets the form when reopened with another name', () => {
      const { rerender } = renderDialog({ open: false, prefilledName: 'Initial Name' });
      rerender(<LabelCreateDialog open onClose={vi.fn()} onSubmit={vi.fn()} prefilledName="New Name" />);
      expect(nameField()).toHaveValue('New Name');
    });
  });
});
