import React from 'react';
import { render, screen, fireEvent, waitFor, act, vi } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import SearchPage from '../SearchPage';
import { documentService } from '../../services/api';

// Mock the API service
const mockDocumentService = {
  enhancedSearch: vi.fn(),
  search: vi.fn(),
  download: vi.fn(),
};

vi.mock('../../services/api', () => ({
  documentService: mockDocumentService,
}));

// Mock SearchGuidance component
vi.mock('../../components/SearchGuidance', () => ({
  default: function MockSearchGuidance({ onExampleClick, compact }: any) {
    return (
      <div data-testid="search-guidance">
        <button onClick={() => onExampleClick?.('test query')}>
          Mock Guidance Example
        </button>
        {compact && <span>Compact Mode</span>}
      </div>
    );
  }
});

// Mock useNavigate
const mockNavigate = vi.fn();
vi.mock('react-router-dom', () => ({
  ...vi.importActual('react-router-dom'),
  useNavigate: () => mockNavigate,
}));

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
        tags: ['test', 'document'],
        created_at: '2023-01-01T00:00:00Z',
        has_ocr_text: true,
        search_rank: 0.85,
        snippets: [
          {
            text: 'This is a test document with important information',
            start_offset: 0,
            end_offset: 48,
            highlight_ranges: [
              { start: 10, end: 14 }
            ]
          }
        ]
      }
    ],
    total: 1,
    query_time_ms: 45,
    suggestions: ['\"test\"', 'test*', 'tag:test']
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

describe('SearchPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockDocumentService.enhancedSearch.mockResolvedValue(mockSearchResponse);
    mockDocumentService.search.mockResolvedValue(mockSearchResponse);
  });

  test('renders search page with prominent search bar', () => {
    renderWithRouter(<SearchPage />);
    
    expect(screen.getByText('Search Documents')).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Search documents by content, filename, or tags/)).toBeInTheDocument();
    expect(screen.getByText('Start searching your documents')).toBeInTheDocument();
  });

  test('displays search tips and examples when no query is entered', () => {
    renderWithRouter(<SearchPage />);
    
    expect(screen.getByText('Search Tips:')).toBeInTheDocument();
    expect(screen.getByText('Try: invoice')).toBeInTheDocument();
    expect(screen.getByText('Try: contract')).toBeInTheDocument();
    expect(screen.getByText('Try: tag:important')).toBeInTheDocument();
  });

  test('performs search when user types in search box', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test query');
    });

    // Wait for debounced search
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: 'test query',
          include_snippets: true,
          snippet_length: 200,
          search_mode: 'simple'
        })
      );
    }, { timeout: 2000 });
  });

  test('displays search results with snippets', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
      expect(screen.getByText(/This is a test document/)).toBeInTheDocument();
      expect(screen.getByText('1 results')).toBeInTheDocument();
      expect(screen.getByText('45ms')).toBeInTheDocument();
    });
  });

  test('shows quick suggestions while typing', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Quick suggestions:')).toBeInTheDocument();
    });
  });
  
  test('shows server suggestions from search results', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Related searches:')).toBeInTheDocument();
      expect(screen.getByText('\"test\"')).toBeInTheDocument();
      expect(screen.getByText('test*')).toBeInTheDocument();
      expect(screen.getByText('tag:test')).toBeInTheDocument();
    });
  });

  test('toggles advanced search options with guidance', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const settingsButton = screen.getByRole('button', { name: /settings/i });
    
    await user.click(settingsButton);
    
    expect(screen.getByText('Search Options')).toBeInTheDocument();
    expect(screen.getByText('Enhanced Search')).toBeInTheDocument();
    expect(screen.getByText('Show Snippets')).toBeInTheDocument();
    expect(screen.getByTestId('search-guidance')).toBeInTheDocument();
    expect(screen.getByText('Compact Mode')).toBeInTheDocument();
  });

  test('changes search mode with simplified labels', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    // Type a search query first to show the search mode selector
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      const phraseButton = screen.getByRole('button', { name: 'Exact phrase' });
      expect(phraseButton).toBeInTheDocument();
    });

    const phraseButton = screen.getByRole('button', { name: 'Exact phrase' });
    await user.click(phraseButton);

    // Wait for search to be called with new mode
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          search_mode: 'phrase'
        })
      );
    });
  });
  
  test('displays simplified search mode labels', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Smart' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Exact phrase' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Similar words' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Advanced' })).toBeInTheDocument();
    });
  });

  test('handles search suggestions click', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Related searches:')).toBeInTheDocument();
    });

    const suggestionChip = screen.getByText('\"test\"');
    await user.click(suggestionChip);

    expect(searchInput.value).toBe('\"test\"');
  });

  test('clears search input', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test query');
    });

    const clearButton = screen.getByRole('button', { name: /clear/i });
    await user.click(clearButton);

    expect(searchInput.value).toBe('');
  });

  test('toggles enhanced search setting', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    // Open advanced options
    const settingsButton = screen.getByRole('button', { name: /settings/i });
    await user.click(settingsButton);
    
    const enhancedSearchSwitch = screen.getByRole('checkbox', { name: /enhanced search/i });
    await user.click(enhancedSearchSwitch);

    // Type a search to trigger API call
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    // Should use regular search instead of enhanced search
    await waitFor(() => {
      expect(documentService.search).toHaveBeenCalled();
    });
  });

  test('changes snippet length setting', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    // Open advanced options
    const settingsButton = screen.getByRole('button', { name: /settings/i });
    await user.click(settingsButton);
    
    const snippetSelect = screen.getByLabelText('Snippet Length');
    await user.click(snippetSelect);
    
    const longOption = screen.getByText('Long (400)');
    await user.click(longOption);

    // Type a search to trigger API call
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          snippet_length: 400
        })
      );
    });
  });

  test('displays enhanced loading state with progress during search', async () => {
    const user = userEvent.setup();
    
    // Mock a delayed response
    documentService.enhancedSearch.mockImplementation(() => 
      new Promise(resolve => setTimeout(() => resolve(mockSearchResponse), 200))
    );
    
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 't');
    });

    // Should show loading indicators
    expect(screen.getAllByRole('progressbar').length).toBeGreaterThan(0);
    
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  test('handles search error gracefully', async () => {
    const user = userEvent.setup();
    
    documentService.enhancedSearch.mockRejectedValue(new Error('Search failed'));
    
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Search failed. Please try again.')).toBeInTheDocument();
    });
  });

  test('navigates to document details on view click', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    const viewButton = screen.getByLabelText('View Details');
    await user.click(viewButton);

    expect(mockNavigate).toHaveBeenCalledWith('/documents/1');
  });

  test('handles document download', async () => {
    const user = userEvent.setup();
    const mockBlob = new Blob(['test content'], { type: 'application/pdf' });
    mockDocumentService.download.mockResolvedValue({ data: mockBlob });
    
    // Mock URL.createObjectURL
    global.URL.createObjectURL = vi.fn(() => 'mock-url');
    global.URL.revokeObjectURL = vi.fn();
    
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    const downloadButton = screen.getByLabelText('Download');
    await user.click(downloadButton);

    expect(documentService.download).toHaveBeenCalledWith('1');
  });

  test('switches between grid and list view modes', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    const listViewButton = screen.getByRole('button', { name: /list view/i });
    await user.click(listViewButton);

    // The view should change (this would be more thoroughly tested with visual regression tests)
    expect(listViewButton).toHaveAttribute('aria-pressed', 'true');
  });

  test('displays file type icons correctly', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      // Should show PDF icon for PDF file
      expect(screen.getByTestId('PictureAsPdfIcon')).toBeInTheDocument();
    });
  });

  test('displays OCR badge when document has OCR text', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('OCR')).toBeInTheDocument();
    });
  });

  test('highlights search terms in snippets', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      // Should render the snippet with highlighted text
      expect(screen.getByText(/This is a test document/)).toBeInTheDocument();
    });
  });

  test('shows relevance score when available', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      expect(screen.getByText('Relevance: 85.0%')).toBeInTheDocument();
    });
  });
});

// New functionality tests
describe('Enhanced Search Features', () => {
  test('shows typing indicator while user is typing', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    // Start typing without completing
    await act(async () => {
      await user.type(searchInput, 't', { delay: 50 });
    });

    // Should show typing indicator
    expect(screen.getAllByRole('progressbar').length).toBeGreaterThan(0);
  });
  
  test('shows improved no results state with suggestions', async () => {
    const user = userEvent.setup();
    
    // Mock empty response
    mockDocumentService.enhancedSearch.mockResolvedValue({
      data: {
        documents: [],
        total: 0,
        query_time_ms: 10,
        suggestions: []
      }
    });
    
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'nonexistent');
    });

    await waitFor(() => {
      expect(screen.getByText(/No results found for "nonexistent"/)).toBeInTheDocument();
      expect(screen.getByText('Suggestions:')).toBeInTheDocument();
      expect(screen.getByText('• Try simpler or more general terms')).toBeInTheDocument();
    });
  });
  
  test('clickable example chips in empty state work correctly', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const invoiceChip = screen.getByText('Try: invoice');
    await user.click(invoiceChip);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    expect(searchInput.value).toBe('invoice');
  });
  
  test('search guidance example click works', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const settingsButton = screen.getByRole('button', { name: /settings/i });
    await user.click(settingsButton);
    
    const guidanceExample = screen.getByText('Mock Guidance Example');
    await user.click(guidanceExample);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    expect(searchInput.value).toBe('test query');
  });
  
  test('mobile filter toggle works', async () => {
    const user = userEvent.setup();
    
    // Mock mobile viewport
    Object.defineProperty(window, 'innerWidth', {
      writable: true,
      configurable: true,
      value: 500,
    });
    
    renderWithRouter(<SearchPage />);
    
    // Mobile filter button should be visible
    const mobileFilterButton = screen.getByTestId('FilterIcon');
    expect(mobileFilterButton).toBeInTheDocument();
  });
  
  test('search results have enhanced CSS classes for styling', async () => {
    const user = userEvent.setup();
    renderWithRouter(<SearchPage />);
    
    const searchInput = screen.getByPlaceholderText(/Search documents by content, filename, or tags/);
    
    await act(async () => {
      await user.type(searchInput, 'test');
    });

    await waitFor(() => {
      const resultCard = screen.getByText('test.pdf').closest('[class*="search-result-card"]');
      expect(resultCard).toBeInTheDocument();
    });
  });
});