import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import LabelSelector from '../LabelSelector';
import { type LabelData } from '../Label';

const theme = createTheme();

const mockLabels: LabelData[] = [
  {
    id: 'label-1',
    name: 'Important',
    description: 'High priority items',
    color: '#d73a49',
    icon: 'star',
    is_system: true,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    document_count: 10,
    source_count: 2,
  },
  {
    id: 'label-2',
    name: 'Work',
    description: 'Work-related documents',
    color: '#0969da',
    icon: 'work',
    is_system: true,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    document_count: 5,
    source_count: 1,
  },
  {
    id: 'label-3',
    name: 'Personal Project',
    description: 'My personal project files',
    color: '#28a745',
    icon: 'folder',
    is_system: false,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    document_count: 3,
    source_count: 0,
  },
];

const renderLabelSelector = (props: Partial<React.ComponentProps<typeof LabelSelector>> = {}) => {
  const defaultProps = {
    selectedLabels: [],
    availableLabels: mockLabels,
    onLabelsChange: vi.fn(),
    ...props,
  };

  return render(
    <ThemeProvider theme={theme}>
      <LabelSelector {...defaultProps} />
    </ThemeProvider>
  );
};

describe('LabelSelector Component', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
  });

  describe('Basic Rendering', () => {
    test('should render autocomplete input', () => {
      renderLabelSelector();
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    test('should show placeholder text', () => {
      renderLabelSelector();
      expect(screen.getByPlaceholderText('Search or create labels...')).toBeInTheDocument();
    });

    test('should show custom placeholder', () => {
      renderLabelSelector({ placeholder: 'Custom placeholder' });
      expect(screen.getByPlaceholderText('Custom placeholder')).toBeInTheDocument();
    });

    test('should render with selected labels', () => {
      const selectedLabels = [mockLabels[0]];
      renderLabelSelector({ selectedLabels });
      
      expect(screen.getByText('Important')).toBeInTheDocument();
    });
  });

  describe('Label Selection', () => {
    test('should call onLabelsChange when label is selected', async () => {
      const onLabelsChange = vi.fn();
      renderLabelSelector({ onLabelsChange });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      // Wait for options to appear and click on one
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      await user.click(screen.getByText('Important'));
      
      expect(onLabelsChange).toHaveBeenCalledWith([mockLabels[0]]);
    });

    test('should filter out already selected labels from options', async () => {
      const selectedLabels = [mockLabels[0]]; // Important is selected
      renderLabelSelector({ selectedLabels });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      // Important should not appear in the dropdown options (but may appear in selected tags)
      // We need to check specifically in the dropdown, not in the entire document
      const dropdownOptions = screen.getByRole('listbox');
      expect(dropdownOptions).toBeInTheDocument();
      
      // Check that Important is not in the dropdown options
      const optionsList = screen.getAllByRole('option');
      const optionTexts = optionsList.map(option => option.textContent);
      expect(optionTexts).not.toContain('Important');
    });

    test('should support single selection mode', async () => {
      const onLabelsChange = vi.fn();
      renderLabelSelector({ 
        onLabelsChange, 
        multiple: false 
      });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      await user.click(screen.getByText('Important'));
      
      expect(onLabelsChange).toHaveBeenCalledWith([mockLabels[0]]);
    });

    test('should support multiple selection mode', async () => {
      const onLabelsChange = vi.fn();
      const selectedLabels = [mockLabels[0]];
      
      renderLabelSelector({ 
        selectedLabels,
        onLabelsChange, 
        multiple: true 
      });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Work')).toBeInTheDocument();
      });
      
      await user.click(screen.getByText('Work'));
      
      expect(onLabelsChange).toHaveBeenCalledWith([mockLabels[0], mockLabels[1]]);
    });
  });

  describe('Label Removal', () => {
    test('should remove label when delete button is clicked', async () => {
      const onLabelsChange = vi.fn();
      // Use only non-system labels since system labels don't have delete buttons
      const selectedLabels = [mockLabels[2]]; // Personal Project (non-system)
      
      renderLabelSelector({ 
        selectedLabels,
        onLabelsChange 
      });
      
      // Find the chip with the delete button
      const personalProjectChip = screen.getByText('Personal Project').closest('.MuiChip-root');
      expect(personalProjectChip).toBeInTheDocument();
      
      // Find the delete button within that specific chip
      const deleteButton = personalProjectChip?.querySelector('[data-testid="CloseIcon"]');
      expect(deleteButton).toBeInTheDocument();
      
      if (deleteButton) {
        await user.click(deleteButton as Element);
      }
      
      expect(onLabelsChange).toHaveBeenCalledWith([]);
    });

    test('should not show delete buttons when disabled', () => {
      const selectedLabels = [mockLabels[2]]; // Non-system label
      
      renderLabelSelector({ 
        selectedLabels,
        disabled: true 
      });
      
      expect(screen.queryByTestId('CloseIcon')).not.toBeInTheDocument();
    });
  });

  describe('Label Grouping', () => {
    test('should group system and user labels', async () => {
      renderLabelSelector();
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        // Check that labels appear in the dropdown
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
    });

    test('should show only system labels when no user labels exist', async () => {
      const systemOnlyLabels = mockLabels.filter(label => label.is_system);
      renderLabelSelector({ availableLabels: systemOnlyLabels });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.queryByText('Personal Project')).not.toBeInTheDocument();
      });
    });
  });

  describe('Search Functionality', () => {
    test('should filter labels based on search input', async () => {
      renderLabelSelector();
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'work');
      
      await waitFor(() => {
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.queryByText('Important')).not.toBeInTheDocument();
        expect(screen.queryByText('Personal Project')).not.toBeInTheDocument();
      });
    });

    test('should filter by description as well as name', async () => {
      renderLabelSelector();
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'priority');
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
    });

    test('should show no options text when no matches found', async () => {
      renderLabelSelector();
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'nonexistent');
      
      await waitFor(() => {
        expect(screen.getByText('No labels match "nonexistent"')).toBeInTheDocument();
      });
    });
  });

  describe('Create New Label', () => {
    test('should show create button when input has new text', async () => {
      const onCreateLabel = vi.fn().mockResolvedValue({
        id: 'new-label',
        name: 'New Label',
        color: '#0969da',
        is_system: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        document_count: 0,
        source_count: 0,
      });
      
      renderLabelSelector({ 
        onCreateLabel,
        showCreateButton: true 
      });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'New Label');
      
      await waitFor(() => {
        // Look for the create buttons - there should be multiple
        expect(screen.getAllByText('Create "New Label"').length).toBeGreaterThan(0);
      });
    });

    test('should not show create button when onCreateLabel is not provided', async () => {
      renderLabelSelector({ showCreateButton: true });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'New Label');
      
      await waitFor(() => {
        expect(screen.queryByText('Create "New Label"')).not.toBeInTheDocument();
      });
    });

    test('should not show create button when showCreateButton is false', async () => {
      const onCreateLabel = vi.fn();
      
      renderLabelSelector({ 
        onCreateLabel,
        showCreateButton: false 
      });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'New Label');
      
      // Should not show create button
      expect(screen.queryByTitle('Create label "New Label"')).not.toBeInTheDocument();
    });

    test('should not show create button for existing label names', async () => {
      const onCreateLabel = vi.fn();
      
      renderLabelSelector({ 
        onCreateLabel,
        showCreateButton: true 
      });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'Important'); // Existing label name
      
      // Should not show create button for existing names
      expect(screen.queryByText('Create "Important"')).not.toBeInTheDocument();
    });

    test('should call onCreateLabel when create button is clicked', async () => {
      const onCreateLabel = vi.fn().mockResolvedValue({
        id: 'new-label',
        name: 'New Label',
        color: '#0969da',
        is_system: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        document_count: 0,
        source_count: 0,
      });
      
      const onLabelsChange = vi.fn();
      
      renderLabelSelector({ 
        onCreateLabel,
        onLabelsChange,
        showCreateButton: true 
      });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'New Label');
      
      await waitFor(() => {
        expect(screen.getAllByText('Create "New Label"').length).toBeGreaterThan(0);
      });
      
      const createButtons = screen.getAllByText('Create "New Label"');
      await user.click(createButtons[0]);
      
      // Wait for dialog to open
      await waitFor(() => {
        expect(screen.getByText('Create New Label')).toBeInTheDocument();
      });
      
      // Submit the form (the name is already pre-filled)
      const createButton = screen.getByRole('button', { name: 'Create' });
      await user.click(createButton);
      
      await waitFor(() => {
        expect(onCreateLabel).toHaveBeenCalledWith(expect.objectContaining({
          name: 'New Label',
          description: undefined,
          color: '#0969da',
          background_color: undefined,
          icon: undefined,
        }));
      });
    });
  });

  describe('Max Tags Limit', () => {
    test('should respect maxTags limit', async () => {
      const onLabelsChange = vi.fn();
      const selectedLabels = [mockLabels[0], mockLabels[1]]; // 2 labels selected
      
      renderLabelSelector({ 
        selectedLabels,
        onLabelsChange,
        maxTags: 2 
      });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      await user.click(screen.getByText('Personal Project'));
      
      // Should not add the third label due to maxTags limit
      expect(onLabelsChange).not.toHaveBeenCalled();
    });

    test('should allow adding labels when under the limit', async () => {
      const onLabelsChange = vi.fn();
      const selectedLabels = [mockLabels[0]]; // 1 label selected
      
      renderLabelSelector({ 
        selectedLabels,
        onLabelsChange,
        maxTags: 2 
      });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Work')).toBeInTheDocument();
      });
      
      await user.click(screen.getByText('Work'));
      
      // Should add the second label as we're under the limit
      expect(onLabelsChange).toHaveBeenCalledWith([mockLabels[0], mockLabels[1]]);
    });
  });

  describe('Disabled State', () => {
    test('should disable input when disabled prop is true', () => {
      renderLabelSelector({ disabled: true });
      
      const input = screen.getByRole('combobox');
      expect(input).toBeDisabled();
    });

    test('should not show create button when disabled', async () => {
      const onCreateLabel = vi.fn();
      
      renderLabelSelector({ 
        onCreateLabel,
        disabled: true,
        showCreateButton: true 
      });
      
      const input = screen.getByRole('combobox');
      // Cannot type when disabled
      expect(input).toBeDisabled();
    });
  });

  describe('Size Variants', () => {
    test('should render with small size', () => {
      renderLabelSelector({ size: 'small' });
      
      const input = screen.getByRole('combobox');
      // The size class is applied to the OutlinedInput root element
      const inputContainer = input.closest('.MuiOutlinedInput-root');
      expect(inputContainer).toHaveClass('MuiInputBase-sizeSmall');
    });

    test('should render with medium size by default', () => {
      renderLabelSelector();
      
      const input = screen.getByRole('combobox');
      expect(input.parentElement?.parentElement).not.toHaveClass('MuiInputBase-sizeSmall');
    });
  });

  describe('Keyboard Navigation', () => {
    test('should support keyboard navigation through options', async () => {
      const onLabelsChange = vi.fn();
      renderLabelSelector({ onLabelsChange });
      
      const input = screen.getByRole('combobox');
      await user.click(input);
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      // Navigate with arrow keys and select with Enter
      await user.keyboard('{ArrowDown}');
      await user.keyboard('{Enter}');
      
      expect(onLabelsChange).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    test('should handle create label error gracefully', async () => {
      const onCreateLabel = vi.fn().mockRejectedValue(new Error('Create failed'));
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      renderLabelSelector({ 
        onCreateLabel,
        showCreateButton: true 
      });
      
      const input = screen.getByRole('combobox');
      await user.type(input, 'New Label');
      
      await waitFor(() => {
        expect(screen.getAllByText('Create "New Label"').length).toBeGreaterThan(0);
      });
      
      const createButtons = screen.getAllByText('Create "New Label"');
      await user.click(createButtons[0]);
      
      // Wait for dialog to open
      await waitFor(() => {
        expect(screen.getByText('Create New Label')).toBeInTheDocument();
      });
      
      // Submit the form to trigger the error
      const createButton = screen.getByRole('button', { name: 'Create' });
      await user.click(createButton);
      
      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('Failed to create label:', expect.any(Error));
      });
      
      consoleError.mockRestore();
    });
  });
});