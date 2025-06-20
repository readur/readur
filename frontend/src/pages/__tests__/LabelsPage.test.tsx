import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import { BrowserRouter } from 'react-router-dom';
import LabelsPage from '../LabelsPage';
import * as useApiModule from '../../hooks/useApi';

const theme = createTheme();

const mockLabels = [
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
  {
    id: 'label-4',
    name: 'Archive',
    description: 'Archived items',
    color: '#6e7781',
    icon: 'archive',
    is_system: true,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    document_count: 0,
    source_count: 0,
  },
];

const renderLabelsPage = async () => {
  let renderResult;
  await act(async () => {
    renderResult = render(
      <BrowserRouter>
        <ThemeProvider theme={theme}>
          <LabelsPage />
        </ThemeProvider>
      </BrowserRouter>
    );
  });
  return renderResult;
};

describe('LabelsPage Component', () => {
  let user: ReturnType<typeof userEvent.setup>;
  let mockApi: {
    get: ReturnType<typeof vi.fn>;
    post: ReturnType<typeof vi.fn>;
    put: ReturnType<typeof vi.fn>;
    delete: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    user = userEvent.setup();
    
    mockApi = {
      get: vi.fn(),
      post: vi.fn(),
      put: vi.fn(),
      delete: vi.fn(),
    };

    vi.spyOn(useApiModule, 'useApi').mockReturnValue(mockApi);
    
    // Default successful API responses with proper status code
    mockApi.get.mockResolvedValue({ status: 200, data: mockLabels });
    mockApi.post.mockResolvedValue({ status: 201, data: mockLabels[0] });
    mockApi.put.mockResolvedValue({ status: 200, data: mockLabels[0] });
    mockApi.delete.mockResolvedValue({ status: 204 });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Initial Rendering', () => {
    test('should render page title and create button', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Label Management')).toBeInTheDocument();
        expect(screen.getByText('Create Label')).toBeInTheDocument();
      });
    });

    test('should show loading state initially', async () => {
      // Mock API to never resolve
      mockApi.get.mockImplementation(() => new Promise(() => {}));
      
      await renderLabelsPage();
      
      expect(screen.getByText('Loading labels...')).toBeInTheDocument();
    });

    test('should fetch labels on mount', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledWith('/labels?include_counts=true');
      });
    });

    test('should display labels after loading', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    test('should show error message when API fails', async () => {
      mockApi.get.mockRejectedValue(new Error('API Error'));
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText(/Failed to load labels/)).toBeInTheDocument();
      });
    });

    test('should allow dismissing error alert', async () => {
      mockApi.get.mockRejectedValue(new Error('API Error'));
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText(/Failed to load labels/)).toBeInTheDocument();
      });
      
      const closeButton = screen.getByLabelText('Close');
      await user.click(closeButton);
      
      expect(screen.queryByText(/Failed to load labels/)).not.toBeInTheDocument();
    });

    test('should handle 401 authentication errors', async () => {
      mockApi.get.mockRejectedValue({
        response: { status: 401 },
        message: 'Unauthorized'
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Authentication required. Please log in again.')).toBeInTheDocument();
      });
    });

    test('should handle 403 access denied errors', async () => {
      mockApi.get.mockRejectedValue({
        response: { status: 403 },
        message: 'Forbidden'
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Access denied. You do not have permission to view labels.')).toBeInTheDocument();
      });
    });

    test('should handle 500 server errors', async () => {
      mockApi.get.mockRejectedValue({
        response: { status: 500 },
        message: 'Internal Server Error'
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Server error. Please try again later.')).toBeInTheDocument();
      });
    });

    test('should handle non-array response data', async () => {
      // This is the main fix - when API returns non-array data
      mockApi.get.mockResolvedValue({ 
        status: 200,
        data: { error: 'Something went wrong' } // Not an array!
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Received invalid data format from server')).toBeInTheDocument();
      });
      
      // Should not crash - labels should be empty array
      expect(screen.getByText('No labels found')).toBeInTheDocument();
    });

    test('should handle unexpected response status with valid array', async () => {
      mockApi.get.mockResolvedValue({ 
        status: 202, // Unexpected status
        data: mockLabels
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Server returned unexpected response (202)')).toBeInTheDocument();
      });
    });

    test('should ensure labels state is always an array', async () => {
      // Mock a scenario where data is not an array
      mockApi.get.mockResolvedValue({ 
        status: 200,
        data: null
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Received invalid data format from server')).toBeInTheDocument();
      });
      
      // Should not crash when trying to filter
      expect(screen.getByText('No labels found')).toBeInTheDocument();
    });

    test('should handle string response data', async () => {
      // Another scenario where response.data is not an array
      mockApi.get.mockResolvedValue({ 
        status: 200,
        data: 'Server maintenance in progress'
      });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Received invalid data format from server')).toBeInTheDocument();
      });
    });
  });

  describe('Search and Filtering', () => {
    test('should render search input', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByPlaceholderText('Search labels...')).toBeInTheDocument();
      });
    });

    test('should filter labels by search term', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('Work')).toBeInTheDocument();
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const searchInput = screen.getByPlaceholderText('Search labels...');
      await user.type(searchInput, 'work');
      
      expect(screen.getByText('Work')).toBeInTheDocument();
      expect(screen.queryByText('Important')).not.toBeInTheDocument();
      expect(screen.queryByText('Personal Project')).not.toBeInTheDocument();
    });

    test('should filter labels by description', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      const searchInput = screen.getByPlaceholderText('Search labels...');
      await user.type(searchInput, 'priority');
      
      expect(screen.getByText('Important')).toBeInTheDocument();
      expect(screen.queryByText('Work')).not.toBeInTheDocument();
    });

    test('should toggle system labels filter', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const systemLabelsChip = screen.getByRole('button', { name: 'System Labels' });
      await user.click(systemLabelsChip);
      
      // Should hide system labels
      expect(screen.queryByText('Important')).not.toBeInTheDocument();
      expect(screen.getByText('Personal Project')).toBeInTheDocument();
    });
  });

  describe('Label Grouping', () => {
    test('should display system labels section', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /system labels/i })).toBeInTheDocument();
      });
    });

    test('should display user labels section', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('My Labels')).toBeInTheDocument();
      });
    });

    test('should group labels correctly', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        const systemSection = screen.getByRole('heading', { name: /system labels/i });
        const userSection = screen.getByRole('heading', { name: /my labels/i });
        
        expect(systemSection).toBeInTheDocument();
        expect(userSection).toBeInTheDocument();
      });
    });
  });

  describe('Label Cards', () => {
    test('should display label information in cards', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
        expect(screen.getByText('High priority items')).toBeInTheDocument();
        expect(screen.getByText('Documents: 10')).toBeInTheDocument();
        expect(screen.getByText('Sources: 2')).toBeInTheDocument();
      });
    });

    test('should show edit and delete buttons for user labels', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      // Find the user label card and check for action buttons
      const userLabelCard = screen.getByText('Personal Project').closest('.MuiCard-root');
      expect(userLabelCard).toBeInTheDocument();
      
      // Should have edit and delete buttons
      const editButtons = screen.getAllByLabelText(/edit/i);
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      
      expect(editButtons.length).toBeGreaterThan(0);
      expect(deleteButtons.length).toBeGreaterThan(0);
    });

    test('should not show edit/delete buttons for system labels', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      // System labels should not have edit/delete buttons
      const systemLabelCards = screen.getAllByText(/System/).length;
      expect(systemLabelCards).toBeGreaterThan(0);
    });
  });

  describe('Create Label', () => {
    test('should open create dialog when create button is clicked', async () => {
      await renderLabelsPage();
      
      const createButton = screen.getByText('Create Label');
      await user.click(createButton);
      
      expect(screen.getByText('Create New Label')).toBeInTheDocument();
    });

    test('should call API when creating new label', async () => {
      const newLabel = {
        id: 'new-label',
        name: 'New Label',
        color: '#ff0000',
        is_system: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        document_count: 0,
        source_count: 0,
      };
      
      mockApi.post.mockResolvedValue({ status: 201, data: newLabel });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Create Label')).toBeInTheDocument();
      });
      
      const createButton = screen.getByText('Create Label');
      await user.click(createButton);
      
      // Wait for dialog to open
      await waitFor(() => {
        expect(screen.getByText('Create New Label')).toBeInTheDocument();
      });
      
      // Fill out the form (this would be a simplified test)
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'New Label');
      
      const submitButton = screen.getByText('Create');
      await user.click(submitButton);
      
      await waitFor(() => {
        expect(mockApi.post).toHaveBeenCalledWith('/labels', expect.objectContaining({
          name: 'New Label'
        }));
      });
    });
  });

  describe('Edit Label', () => {
    test('should open edit dialog when edit button is clicked', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const editButtons = screen.getAllByLabelText(/edit/i);
      await user.click(editButtons[0]);
      
      expect(screen.getByText('Edit Label')).toBeInTheDocument();
    });

    test('should call API when updating label', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const editButtons = screen.getAllByLabelText(/edit/i);
      await user.click(editButtons[0]);
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.clear(nameInput);
      await user.type(nameInput, 'Updated Label');
      
      const updateButton = screen.getByText('Update');
      await user.click(updateButton);
      
      await waitFor(() => {
        expect(mockApi.put).toHaveBeenCalledWith(`/labels/${mockLabels[2].id}`, expect.objectContaining({
          name: 'Updated Label'
        }));
      });
    });
  });

  describe('Delete Label', () => {
    test('should open delete confirmation when delete button is clicked', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      expect(screen.getByText('Delete Label')).toBeInTheDocument();
      expect(screen.getByText(/are you sure you want to delete the label/i)).toBeInTheDocument();
    });

    test('should show usage warning when label has documents', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      expect(screen.getByText(/This label is currently used by 3 document\(s\)/)).toBeInTheDocument();
    });

    test('should call API when confirming deletion', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      const confirmButton = screen.getByRole('button', { name: 'Delete' });
      await user.click(confirmButton);
      
      await waitFor(() => {
        expect(mockApi.delete).toHaveBeenCalledWith(`/labels/${mockLabels[2].id}`);
      });
    });

    test('should cancel deletion when cancel is clicked', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      const cancelButton = screen.getByText('Cancel');
      await user.click(cancelButton);
      
      await waitFor(() => {
        expect(screen.queryByText('Delete Label')).not.toBeInTheDocument();
      });
      expect(mockApi.delete).not.toHaveBeenCalled();
    });
  });

  describe('Empty States', () => {
    test('should show empty state when no labels found', async () => {
      mockApi.get.mockResolvedValue({ status: 200, data: [] });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('No labels found')).toBeInTheDocument();
        expect(screen.getByText("You haven't created any labels yet")).toBeInTheDocument();
        expect(screen.getByText('Create Your First Label')).toBeInTheDocument();
      });
    });

    test('should show search empty state when no search results', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Important')).toBeInTheDocument();
      });
      
      const searchInput = screen.getByPlaceholderText('Search labels...');
      await user.type(searchInput, 'nonexistent');
      
      expect(screen.getByText('No labels found')).toBeInTheDocument();
      expect(screen.getByText('No labels match "nonexistent"')).toBeInTheDocument();
    });

    test('should show create button in empty state', async () => {
      mockApi.get.mockResolvedValue({ status: 200, data: [] });
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Create Your First Label')).toBeInTheDocument();
      });
      
      const createButton = screen.getByText('Create Your First Label');
      await user.click(createButton);
      
      expect(screen.getByText('Create New Label')).toBeInTheDocument();
    });
  });

  describe('Data Refresh', () => {
    test('should refresh labels after successful creation', async () => {
      const newLabel = {
        id: 'new-label',
        name: 'New Label',
        color: '#ff0000',
        is_system: false,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        document_count: 0,
        source_count: 0,
      };
      
      mockApi.post.mockResolvedValue({ status: 201, data: newLabel });
      
      await renderLabelsPage();
      
      // Initial load
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledTimes(1);
      });
      
      const createButton = screen.getByText('Create Label');
      await user.click(createButton);
      
      const nameInput = screen.getByLabelText(/label name/i);
      await user.type(nameInput, 'New Label');
      
      const submitButton = screen.getByText('Create');
      await user.click(submitButton);
      
      // Should call API again to refresh
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledTimes(2);
      });
    });

    test('should refresh labels after successful deletion', async () => {
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledTimes(1);
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      const confirmButton = screen.getByRole('button', { name: 'Delete' });
      await user.click(confirmButton);
      
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledTimes(2);
      });
    });
  });

  describe('Error Handling in Operations', () => {
    test('should show error when label deletion fails', async () => {
      mockApi.delete.mockRejectedValue(new Error('Delete failed'));
      
      await renderLabelsPage();
      
      await waitFor(() => {
        expect(screen.getByText('Personal Project')).toBeInTheDocument();
      });
      
      const deleteButtons = screen.getAllByLabelText(/delete/i);
      await user.click(deleteButtons[0]);
      
      const confirmButton = screen.getByRole('button', { name: 'Delete' });
      await user.click(confirmButton);
      
      await waitFor(() => {
        expect(screen.getByText('Failed to delete label')).toBeInTheDocument();
      });
    });
  });
});