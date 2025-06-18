import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';
import GlobalSearchBar from '../GlobalSearchBar';
import { documentService } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  documentService: {
    enhancedSearch: vi.fn(),
  }
}));

// Mock useNavigate
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};
global.localStorage = localStorageMock;

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
      {
        id: '2',
        filename: 'image.png',
        original_filename: 'image.png',
        file_size: 2048,
        mime_type: 'image/png',
        tags: ['image'],
        created_at: '2023-01-02T00:00:00Z',
        has_ocr_text: false,
        search_rank: 0.75,
      }
    ],
    total: 2,
  }
};

// Helper to render component with router
const renderWithRouter = (component) => {
  return render(
    <BrowserRouter>
      {component}
    </BrowserRouter>
  );
};

describe('GlobalSearchBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock.getItem.mockReturnValue(null);
    (documentService.enhancedSearch as any).mockResolvedValue(mockSearchResponse);
  });

  test('renders search input with placeholder', () => {
    renderWithRouter(<GlobalSearchBar />);
    
    expect(screen.getByPlaceholderText('Search documents...')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  test('shows popular searches when input is focused', async () => {
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      searchInput.focus();
    });

    await waitFor(() => {
      expect(screen.getByText('Start typing to search documents')).toBeInTheDocument();
      expect(screen.getByText('Popular searches:')).toBeInTheDocument();
      expect(screen.getByText('invoice')).toBeInTheDocument();
      expect(screen.getByText('contract')).toBeInTheDocument();
      expect(screen.getByText('report')).toBeInTheDocument();
    });
  });

  test('performs search when user types', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith({
        query: 'test',
        limit: 5,
        include_snippets: false,
        search_mode: 'simple',
      });
    }, { timeout: 2000 });
  });

  test('displays search results', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Quick Results')).toBeInTheDocument();
      expect(screen.getByText('2 found')).toBeInTheDocument(); // Enhanced result count display
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
      expect(screen.getByText('image.png')).toBeInTheDocument();
    });
  });

  test('shows file type icons for different document types', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      // Should show PDF icon for PDF file
      expect(screen.getByTestId('PictureAsPdfIcon')).toBeInTheDocument();
      // Should show Image icon for image file
      expect(screen.getByTestId('ImageIcon')).toBeInTheDocument();
    });
  });

  test('shows OCR badge when document has OCR text', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('OCR')).toBeInTheDocument();
    });
  });

  test('shows relevance score for documents', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('85%')).toBeInTheDocument();
      expect(screen.getByText('75%')).toBeInTheDocument();
    });
  });

  test('navigates to document when result is clicked', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    const documentLink = screen.getByText('test.pdf').closest('li');
    await user.click(documentLink);

    expect(mockNavigate).toHaveBeenCalledWith('/documents/1');
  });

  test('navigates to full search page on Enter key', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test query');
      await user.keyboard('{Enter}');
    });

    expect(mockNavigate).toHaveBeenCalledWith('/search?q=test%20query');
  });

  test('clears input when clear button is clicked', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    const clearButton = screen.getByRole('button', { name: /clear/i });
    await user.click(clearButton);

    expect(searchInput.value).toBe('');
  });

  test('hides results when clicking away', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Quick Results')).toBeInTheDocument();
    });

    // Click outside the component
    await user.click(document.body);

    await waitFor(() => {
      expect(screen.queryByText('Quick Results')).not.toBeInTheDocument();
    });
  });

  test('shows "View all results" link when there are many results', async () => {
    // Mock response with 5 or more results to trigger the link
    (documentService.enhancedSearch as any).mockResolvedValue({
      data: {
        documents: Array.from({ length: 5 }, (_, i) => ({
          id: `${i + 1}`,
          filename: `test${i + 1}.pdf`,
          original_filename: `test${i + 1}.pdf`,
          file_size: 1024,
          mime_type: 'application/pdf',
          tags: ['test'],
          created_at: '2023-01-01T00:00:00Z',
          has_ocr_text: true,
          search_rank: 0.85,
        })),
        total: 10,
      }
    });

    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText(/View all results for "test"/)).toBeInTheDocument();
    });
  });

  test('displays recent searches when no query is entered', async () => {
    // Mock localStorage with recent searches
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['previous search', 'another search']));

    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      searchInput.focus();
    });

    await waitFor(() => {
      expect(screen.getByText('Recent Searches')).toBeInTheDocument();
      expect(screen.getByText('previous search')).toBeInTheDocument();
      expect(screen.getByText('another search')).toBeInTheDocument();
    });
  });

  test('saves search to recent searches when navigating', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'new search');
      await user.keyboard('{Enter}');
    });

    expect(localStorageMock.setItem).toHaveBeenCalledWith(
      'recentSearches',
      JSON.stringify(['new search'])
    );
  });

  test('handles search errors gracefully', async () => {
    documentService.enhancedSearch.mockRejectedValue(new Error('Search failed'));

    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    // Should not crash and should show no results
    await waitFor(() => {
      expect(screen.getByText('No documents found')).toBeInTheDocument();
    });
  });

  test('shows loading state during search', async () => {
    const user = userEvent.setup();
    
    // Mock a delayed response
    documentService.enhancedSearch.mockImplementation(() => 
      new Promise(resolve => setTimeout(() => resolve(mockSearchResponse), 100))
    );
    
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    // Should show loading indicator
    expect(screen.getByText('Searching...')).toBeInTheDocument();
    
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });
  });

  test('formats file sizes correctly', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('1 KB')).toBeInTheDocument(); // 1024 bytes = 1 KB
      expect(screen.getByText('2 KB')).toBeInTheDocument(); // 2048 bytes = 2 KB
    });
  });

  test('closes dropdown on Escape key', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Quick Results')).toBeInTheDocument();
    });

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByText('Quick Results')).not.toBeInTheDocument();
    });
  });

  // New tests for enhanced functionality
  test('shows typing indicator while user is typing', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 't', { delay: 50 });
    });

    // Should show typing indicator
    expect(screen.getAllByRole('progressbar').length).toBeGreaterThan(0);
  });
  
  test('shows smart suggestions while typing', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      await user.type(searchInput, 'inv');
    });

    await waitFor(() => {
      expect(screen.getByText('Try these suggestions:')).toBeInTheDocument();
    });
  });
  
  test('popular search chips are clickable', async () => {
    const user = userEvent.setup();
    renderWithRouter(<GlobalSearchBar />);
    
    const searchInput = screen.getByPlaceholderText('Search documents...');
    
    await act(async () => {
      searchInput.focus();
    });

    await waitFor(() => {
      expect(screen.getByText('invoice')).toBeInTheDocument();
    });

    const invoiceChip = screen.getByText('invoice');
    await user.click(invoiceChip);

    expect(searchInput.value).toBe('invoice');
    expect(mockNavigate).toHaveBeenCalledWith('/search?q=invoice');
  });
});