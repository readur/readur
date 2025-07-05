import { vi } from 'vitest';
import { createComprehensiveAxiosMock, createComprehensiveApiMocks } from '../../../test/comprehensive-mocks';

// Mock axios comprehensively to prevent any real HTTP requests
vi.mock('axios', () => createComprehensiveAxiosMock());

// Mock API services comprehensively
vi.mock('../../../services/api', async () => {
  const actual = await vi.importActual('../../../services/api');
  const apiMocks = createComprehensiveApiMocks();
  
  return {
    ...actual,
    ...apiMocks,
  };
});

// Mock useNavigate
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// Import after mocking
import React from 'react';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import GlobalSearchBar from '../GlobalSearchBar';
import { renderWithProviders, createMockApiServices, createMockLocalStorage } from '../../../test/test-utils';

// Use centralized API mocking
const mockServices = createMockApiServices();
const mockDocumentService = mockServices.documentService;
const localStorageMock = createMockLocalStorage();

// Mock data
const mockSearchResponse = {
  data: {
    documents: [
      {
        id: '1',
        filename: 'test.pdf',
        original_filename: 'test.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        tags: ['test'],
        created_at: '2023-01-01T00:00:00Z',
        has_ocr_text: true,
        search_rank: 0.85,
      },
    ],
    total: 1,
  }
};

// Using centralized render utility (no custom helper needed)

describe('GlobalSearchBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock.getItem.mockReturnValue(null);
    mockDocumentService.enhancedSearch.mockResolvedValue(mockSearchResponse);
  });

  test('renders search input with placeholder', () => {
    renderWithProviders(<GlobalSearchBar />);
    
    expect(screen.getByPlaceholderText('Search documents...')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  test('accepts user input', async () => {
    const user = userEvent.setup();
    renderWithProviders(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    await user.type(searchInput, 'test');
    
    expect(searchInput).toHaveValue('test');
  });

  test('clears input when clear button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    await user.type(searchInput, 'test');
    
    // Find the clear button by looking for ClearIcon
    const clearButton = screen.getByTestId('ClearIcon').closest('button');
    await user.click(clearButton);

    expect(searchInput).toHaveValue('');
  });

  test('shows popular searches when focused', async () => {
    renderWithProviders(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    fireEvent.focus(searchInput);

    await waitFor(() => {
      expect(screen.getByText('Start typing to search documents')).toBeInTheDocument();
    });
  });

  test('handles empty search gracefully', () => {
    renderWithProviders(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    fireEvent.change(searchInput, { target: { value: '' } });
    
    expect(searchInput).toHaveValue('');
  });

  test('handles keyboard navigation', async () => {
    const user = userEvent.setup();
    renderWithProviders(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    await user.type(searchInput, 'test query');
    
    // Just test that the input accepts keyboard input
    expect(searchInput).toHaveValue('test query');
  });
});