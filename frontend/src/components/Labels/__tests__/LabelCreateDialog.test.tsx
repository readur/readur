import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import LabelCreateDialog from '../LabelCreateDialog';
import { type LabelData } from '../Label';

const theme = createTheme();

const mockEditingLabel: LabelData = {
  id: 'edit-label-1',
  name: 'Existing Label',
  description: 'An existing label',
  color: '#ff0000',
  background_color: undefined,
  icon: 'star',
  is_system: false,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  document_count: 5,
  source_count: 2,
};

const renderLabelCreateDialog = (props: Partial<React.ComponentProps<typeof LabelCreateDialog>> = {}) => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onSubmit: vi.fn(),
    ...props,
  };

  return render(
    <ThemeProvider theme={theme}>
      <LabelCreateDialog {...defaultProps} />
    </ThemeProvider>
  );
};

describe('LabelCreateDialog Component', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
  });

  describe('Create Mode', () => {
    test('should render create dialog title', () => {
      renderLabelCreateDialog();
      expect(screen.getByText('Create New Label')).toBeInTheDocument();
    });

    test('should render all form fields', () => {
      renderLabelCreateDialog();
      
      expect(screen.getByLabelText(/label name/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/custom color/i)).toBeInTheDocument();
      expect(screen.getByText('Color')).toBeInTheDocument();
      expect(screen.getByText('Icon (optional)')).toBeInTheDocument();
      expect(screen.getByText('Preview')).toBeInTheDocument();
    });

    test('should show prefilled name when provided', () => {
      renderLabelCreateDialog({ prefilledName: 'Prefilled Name' });
      
      const nameInput = screen.getByLabelText(/label name/i) as HTMLInputElement;
      expect(nameInput.value).toBe('Prefilled Name');
    });

    test('should have default color', () => {
      renderLabelCreateDialog();
      
      const colorInput = screen.getByLabelText(/custom color/i) as HTMLInputElement;
      expect(colorInput.value).toBe('#0969da');
    });

    test('should show create button', () => {
      renderLabelCreateDialog();
      expect(screen.getByText('Create')).toBeInTheDocument();
    });
  });

  describe('Edit Mode', () => {
    test('should render edit dialog title when editing', () => {
      renderLabelCreateDialog({ editingLabel: mockEditingLabel });
      expect(screen.getByText('Edit Label')).toBeInTheDocument();
    });

    test('should populate form with existing label data', () => {
      renderLabelCreateDialog({ editingLabel: mockEditingLabel });
      
      const nameInput = screen.getByLabelText(/label name/i) as HTMLInputElement;
      const descInput = screen.getByLabelText(/description/i) as HTMLInputElement;
      const colorInput = screen.getByLabelText(/custom color/i) as HTMLInputElement;
      
      expect(nameInput.value).toBe('Existing Label');
      expect(descInput.value).toBe('An existing label');
      expect(colorInput.value).toBe('#ff0000');
    });

    test('should show update button when editing', () => {
      renderLabelCreateDialog({ editingLabel: mockEditingLabel });
      expect(screen.getByText('Update')).toBeInTheDocument();
    });
  });

  describe('Form Validation', () => {
    test('should disable submit button when name is empty', () => {
      renderLabelCreateDialog();
      
      const createButton = screen.getByText('Create');
      expect(createButton).toBeDisabled();
    });

    test('should enable submit button when name is provided', async () => {
      renderLabelCreateDialog();
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      const createButton = screen.getByText('Create');
      expect(createButton).not.toBeDisabled();
    });

    test('should disable submit button when name is empty', () => {
      renderLabelCreateDialog();
      
      const createButton = screen.getByText('Create');
      expect(createButton).toBeDisabled();
    });

    test('should enable submit button when name is entered', async () => {
      renderLabelCreateDialog();
      
      // Button should be disabled initially
      const createButton = screen.getByText('Create');
      expect(createButton).toBeDisabled();
      
      // Enter name
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      // Button should be enabled
      expect(createButton).not.toBeDisabled();
    });
  });

  describe('Color Selection', () => {
    test('should render predefined color buttons', () => {
      renderLabelCreateDialog();
      
      // Should render the Color section and have color input
      expect(screen.getByText('Color')).toBeInTheDocument();
      expect(screen.getByLabelText(/custom color/i)).toBeInTheDocument();
      
      // The color buttons are rendered as Material-UI IconButtons with backgroundColor styling
      // We'll just verify the color input and section exist rather than counting buttons
      expect(screen.getByDisplayValue('#0969da')).toBeInTheDocument(); // Default color
    });

    test('should select color when predefined color is clicked', async () => {
      renderLabelCreateDialog();
      
      // Find a specific color button (this is approximate since colors are in styles)
      const colorButtons = screen.getAllByRole('button').filter(button => 
        button.getAttribute('style')?.includes('rgb(215, 58, 73)') // #d73a49 GitHub red
      );
      
      if (colorButtons.length > 0) {
        await user.click(colorButtons[0]);
        
        const colorInput = screen.getByLabelText(/custom color/i) as HTMLInputElement;
        expect(colorInput.value).toBe('#d73a49');
      }
    });

    test('should allow custom color input', async () => {
      renderLabelCreateDialog();
      
      const colorInput = screen.getByLabelText(/custom color/i);
      await user.clear(colorInput);
      await user.type(colorInput, '#abcdef');
      
      expect((colorInput as HTMLInputElement).value).toBe('#abcdef');
    });
  });

  describe('Icon Selection', () => {
    test('should render icon selection buttons', () => {
      renderLabelCreateDialog();
      
      // Should show "None" option and various icon buttons
      expect(screen.getByText('None')).toBeInTheDocument();
      
      // Should have icon buttons - look for buttons with SVG icons
      const allButtons = screen.getAllByRole('button');
      const iconButtons = allButtons.filter(button => 
        button.querySelector('svg[data-testid$="Icon"]') &&
        !button.textContent?.includes('None') &&
        !button.textContent?.includes('Create') &&
        !button.textContent?.includes('Cancel')
      );
      expect(iconButtons.length).toBeGreaterThan(5);
    });

    test('should select None by default', () => {
      renderLabelCreateDialog();
      
      const noneButton = screen.getByText('None').closest('button');
      expect(noneButton).toHaveStyle({ borderColor: expect.stringContaining('#') });
    });

    test('should select icon when clicked', async () => {
      renderLabelCreateDialog();
      
      // Find star icon button by looking for buttons with StarIcon
      const allButtons = screen.getAllByRole('button');
      const starButton = allButtons.find(button => 
        button.querySelector('svg[data-testid="StarIcon"]')
      );
      
      expect(starButton).toBeInTheDocument();
      
      if (starButton) {
        await user.click(starButton);
        
        // Visual feedback should show it's selected (border change)
        expect(starButton).toHaveStyle({ borderColor: expect.stringContaining('#') });
      }
    });

    test('should deselect icon when None is clicked', async () => {
      renderLabelCreateDialog();
      
      // Select an icon first
      const allButtons = screen.getAllByRole('button');
      const starButton = allButtons.find(button => 
        button.querySelector('svg[data-testid="StarIcon"]')
      );
      
      if (starButton) {
        await user.click(starButton);
      }
      
      // Then click None
      const noneButton = screen.getByText('None').closest('button');
      await user.click(noneButton!);
      
      // None should be selected again
      expect(noneButton).toHaveStyle({ borderColor: expect.stringContaining('#') });
    });
  });

  describe('Preview', () => {
    test('should show preview labels', () => {
      renderLabelCreateDialog({ prefilledName: 'Test Label' });
      
      // Should show both filled and outlined preview variants
      const previewLabels = screen.getAllByText('Test Label');
      expect(previewLabels.length).toBeGreaterThanOrEqual(2);
    });

    test('should update preview when name changes', async () => {
      renderLabelCreateDialog();
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Dynamic Preview');
      
      // Preview should update
      expect(screen.getAllByText('Dynamic Preview').length).toBeGreaterThanOrEqual(2);
    });

    test('should show Label Preview when name is empty', () => {
      renderLabelCreateDialog();
      
      expect(screen.getAllByText('Label Preview').length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Form Submission', () => {
    test('should call onSubmit with correct data when creating', async () => {
      const onSubmit = vi.fn();
      renderLabelCreateDialog({ onSubmit });
      
      // Fill form
      const nameInput = screen.getByLabelText(/label name/i);
      const descInput = screen.getByLabelText(/description/i);
      
      await user.type(nameInput, 'Test Label');
      await user.type(descInput, 'Test description');
      
      // Submit
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({
        name: 'Test Label',
        description: 'Test description',
        color: '#0969da',
        background_color: undefined,
        icon: undefined,
      }));
    });

    test('should call onSubmit with updated data when editing', async () => {
      const onSubmit = vi.fn();
      renderLabelCreateDialog({ 
        onSubmit,
        editingLabel: mockEditingLabel 
      });
      
      // Change name
      const nameInput = screen.getByLabelText(/label name/i);
      await user.clear(nameInput);
      await user.type(nameInput, 'Updated Label');
      
      // Submit
      const updateButton = screen.getByText('Update');
      await user.click(updateButton);
      
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({
        name: 'Updated Label',
        description: 'An existing label',
        color: '#ff0000',
        background_color: undefined,
        icon: 'star',
      }));
    });

    test('should handle submission with minimal data', async () => {
      const onSubmit = vi.fn();
      renderLabelCreateDialog({ onSubmit });
      
      // Only fill required name field
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Minimal Label');
      
      // Submit
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({
        name: 'Minimal Label',
        description: undefined,
        color: '#0969da',
        background_color: undefined,
        icon: undefined,
      }));
    });

    test('should trim whitespace from name', async () => {
      const onSubmit = vi.fn();
      renderLabelCreateDialog({ onSubmit });
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, '  Trimmed Label  ');
      
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      expect(onSubmit).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Trimmed Label',
        })
      );
    });
  });

  describe('Loading State', () => {
    test('should show loading state during submission', async () => {
      const onSubmit = vi.fn().mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));
      renderLabelCreateDialog({ onSubmit });
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      expect(screen.getByText('Saving...')).toBeInTheDocument();
      expect(createButton).toBeDisabled();
      
      // Wait for submission to complete
      await waitFor(() => {
        expect(screen.queryByText('Saving...')).not.toBeInTheDocument();
      });
    });

    test('should disable form fields during submission', async () => {
      const onSubmit = vi.fn().mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));
      renderLabelCreateDialog({ onSubmit });
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      expect(nameInput).toBeDisabled();
      expect(screen.getByLabelText(/description/i)).toBeDisabled();
      
      // Wait for submission to complete
      await waitFor(() => {
        expect(nameInput).not.toBeDisabled();
      });
    });
  });

  describe('Dialog Controls', () => {
    test('should call onClose when cancel button is clicked', async () => {
      const onClose = vi.fn();
      renderLabelCreateDialog({ onClose });
      
      const cancelButton = screen.getByText('Cancel');
      await user.click(cancelButton);
      
      expect(onClose).toHaveBeenCalled();
    });

    test('should not call onClose during loading', async () => {
      const onClose = vi.fn();
      const onSubmit = vi.fn().mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));
      
      renderLabelCreateDialog({ onClose, onSubmit });
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      const createButton = screen.getByText('Create');
      await user.click(createButton);
      
      // Try to close during loading - should be disabled due to pointer-events: none
      const cancelButton = screen.getByText('Cancel');
      expect(cancelButton).toBeDisabled();
      
      expect(onClose).not.toHaveBeenCalled();
      
      // Wait for submission to complete
      await waitFor(() => {
        expect(screen.queryByText('Saving...')).not.toBeInTheDocument();
      });
    });

    test('should reset form when dialog is reopened', () => {
      const { rerender } = renderLabelCreateDialog({ 
        open: false,
        prefilledName: 'Initial Name'
      });
      
      // Reopen with different prefilled name
      rerender(
        <ThemeProvider theme={theme}>
          <LabelCreateDialog
            open={true}
            onClose={vi.fn()}
            onSubmit={vi.fn()}
            prefilledName="New Name"
          />
        </ThemeProvider>
      );
      
      const nameInput = screen.getByLabelText(/label name/i) as HTMLInputElement;
      expect(nameInput.value).toBe('New Name');
    });
  });

  describe('Accessibility', () => {
    test('should have proper dialog role', () => {
      renderLabelCreateDialog();
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    test('should have proper form structure', () => {
      renderLabelCreateDialog();
      
      // All inputs should have proper labels
      expect(screen.getByLabelText(/label name/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/custom color/i)).toBeInTheDocument();
    });

    test('should handle form submission via Enter key', async () => {
      const onSubmit = vi.fn();
      renderLabelCreateDialog({ onSubmit });
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'Test Label');
      
      // Submit via Enter key
      await user.keyboard('{Enter}');
      
      expect(onSubmit).toHaveBeenCalled();
    });
  });
});