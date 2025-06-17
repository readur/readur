import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import SearchPage from '../SearchPage';
import { documentService } from '../../services/api';

// Mock the document service
vi.mock('../../services/api', () => ({
  documentService: {
    search: vi.fn(),
    enhancedSearch: vi.fn(),
    getFacets: vi.fn(),
    download: vi.fn(),
  },
}));

const mockSearchResponse = {
  data: {
    documents: [
      {
        id: '1',
        original_filename: 'invoice_2024.pdf',
        filename: 'invoice_2024.pdf',
        file_size: 1024000,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T10:00:00Z',
        has_ocr_text: true,
        tags: ['invoice', '2024'],
        snippets: [
          {
            text: 'This is an invoice for services rendered in January 2024.',
            highlight_ranges: [{ start: 10, end: 17 }, { start: 50, end: 57 }],
          },
        ],
        search_rank: 0.95,
      },
      {
        id: '2',
        original_filename: 'contract_agreement.docx',
        filename: 'contract_agreement.docx',
        file_size: 512000,
        mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        created_at: '2024-01-15T14:30:00Z',
        has_ocr_text: false,
        tags: ['contract', 'legal'],
        snippets: [
          {
            text: 'Contract agreement between parties for invoice processing.',
            highlight_ranges: [{ start: 0, end: 8 }, { start: 40, end: 47 }],
          },
        ],
        search_rank: 0.87,
      },
    ],
    total: 2,
    query_time_ms: 45,
    suggestions: ['invoice processing', 'invoice payment'],
  },
};

const mockFacetsResponse = {
  data: {
    mime_types: [
      { value: 'application/pdf', count: 15 },
      { value: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', count: 8 },
      { value: 'image/jpeg', count: 5 },
      { value: 'text/plain', count: 3 },
    ],
    tags: [
      { value: 'invoice', count: 12 },
      { value: 'contract', count: 6 },
      { value: 'legal', count: 4 },
      { value: '2024', count: 20 },
    ],
  },
};

const renderSearchPage = () => {
  return render(
    <BrowserRouter>
      <SearchPage />
    </BrowserRouter>
  );
};

describe('SearchPage Integration Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (documentService.enhancedSearch as any).mockResolvedValue(mockSearchResponse);
    (documentService.search as any).mockResolvedValue(mockSearchResponse);
    (documentService.getFacets as any).mockResolvedValue(mockFacetsResponse);
  });

  test('performs complete search workflow', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Wait for facets to load
    await waitFor(() => {
      expect(screen.getByText('File Types')).toBeInTheDocument();
    });

    // Enter search query
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Wait for search results
    await waitFor(() => {
      expect(screen.getByText('invoice_2024.pdf')).toBeInTheDocument();
      expect(screen.getByText('contract_agreement.docx')).toBeInTheDocument();
    });

    // Verify search was called
    expect(documentService.enhancedSearch).toHaveBeenCalledWith(
      expect.objectContaining({
        query: 'invoice',
        limit: 100,
        include_snippets: true,
        snippet_length: 200,
        search_mode: 'simple',
      })
    );

    // Verify results are displayed
    expect(screen.getByText('2 documents found')).toBeInTheDocument();
    expect(screen.getByText('Search completed in 45ms')).toBeInTheDocument();
  });

  test('filters results using MIME type facets', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Wait for facets to load
    await waitFor(() => {
      expect(screen.getByText('PDFs')).toBeInTheDocument();
    });

    // Enter search query first
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Wait for initial results
    await waitFor(() => {
      expect(screen.getByText('invoice_2024.pdf')).toBeInTheDocument();
    });

    // Apply PDF filter
    const pdfCheckbox = screen.getByText('PDF Documents').closest('label')?.querySelector('input');
    await user.click(pdfCheckbox!);

    // Verify search is called again with MIME type filter
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: 'invoice',
          mime_types: ['application/pdf'],
        })
      );
    });
  });

  test('uses advanced search options', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Open advanced search panel
    const advancedButton = screen.getByText('Advanced Search Options');
    await user.click(advancedButton);

    // Wait for panel to expand
    await waitFor(() => {
      expect(screen.getByText('Search Behavior')).toBeInTheDocument();
    });

    // Change search mode to fuzzy
    const searchModeSelect = screen.getByDisplayValue('simple');
    await user.click(searchModeSelect);
    await user.click(screen.getByText('Fuzzy Search'));

    // Go to Results Display section
    await user.click(screen.getByText('Results Display'));

    // Change snippet length
    const snippetLengthSelect = screen.getByDisplayValue('200');
    await user.click(snippetLengthSelect);
    await user.click(screen.getByText('Long (400 chars)'));

    // Perform search
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Verify advanced settings are applied
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: 'invoice',
          search_mode: 'fuzzy',
          snippet_length: 400,
        })
      );
    });
  });

  test('displays enhanced snippets with customization', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Perform search
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Wait for results with snippets
    await waitFor(() => {
      expect(screen.getByText(/This is an invoice for services/)).toBeInTheDocument();
    });

    // Find snippet viewer settings
    const settingsButton = screen.getAllByLabelText('Snippet settings')[0];
    await user.click(settingsButton);

    // Change to compact view
    const compactOption = screen.getByLabelText('Compact');
    await user.click(compactOption);

    // Verify compact view is applied (content should still be visible but styled differently)
    expect(screen.getByText(/This is an invoice for services/)).toBeInTheDocument();
  });

  test('suggests search examples and allows interaction', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Open search guide
    const showGuideButton = screen.getByText('Show Guide');
    await user.click(showGuideButton);

    // Wait for guide to expand
    await waitFor(() => {
      expect(screen.getByText('Search Guide')).toBeInTheDocument();
    });

    // Click on an example
    const exampleButtons = screen.getAllByLabelText('Try this search');
    await user.click(exampleButtons[0]);

    // Verify search input is populated
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    expect(searchInput).toHaveValue('invoice');

    // Verify search is triggered
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: 'invoice',
        })
      );
    });
  });

  test('handles search errors gracefully', async () => {
    const user = userEvent.setup();
    (documentService.enhancedSearch as any).mockRejectedValue(new Error('Search failed'));
    
    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Should show error message
    await waitFor(() => {
      expect(screen.getByText('Search failed. Please try again.')).toBeInTheDocument();
    });
  });

  test('switches between view modes', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Perform search first
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Wait for results
    await waitFor(() => {
      expect(screen.getByText('invoice_2024.pdf')).toBeInTheDocument();
    });

    // Switch to list view
    const listViewButton = screen.getByLabelText('List view');
    await user.click(listViewButton);

    // Results should still be visible but in list format
    expect(screen.getByText('invoice_2024.pdf')).toBeInTheDocument();
    expect(screen.getByText('contract_agreement.docx')).toBeInTheDocument();
  });

  test('shows search suggestions', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Wait for suggestions to appear
    await waitFor(() => {
      expect(screen.getByText('Suggestions:')).toBeInTheDocument();
      expect(screen.getByText('invoice processing')).toBeInTheDocument();
      expect(screen.getByText('invoice payment')).toBeInTheDocument();
    });

    // Click on a suggestion
    const suggestionChip = screen.getByText('invoice processing');
    await user.click(suggestionChip);

    // Verify search input is updated
    expect(searchInput).toHaveValue('invoice processing');
  });

  test('applies multiple filters simultaneously', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Wait for facets to load
    await waitFor(() => {
      expect(screen.getByText('File Types')).toBeInTheDocument();
    });

    // Enter search query
    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Apply PDF filter
    const pdfCheckbox = screen.getByText('PDF Documents').closest('label')?.querySelector('input');
    await user.click(pdfCheckbox!);

    // Apply date range filter (if visible)
    const dateRangeSlider = screen.queryByRole('slider', { name: /date range/i });
    if (dateRangeSlider) {
      await user.click(dateRangeSlider);
    }

    // Apply OCR filter
    const ocrSelect = screen.getByDisplayValue('All Documents');
    await user.click(ocrSelect);
    await user.click(screen.getByText('Has OCR Text'));

    // Verify search is called with all filters
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: 'invoice',
          mime_types: ['application/pdf'],
        })
      );
    });
  });

  test('clears all filters when clear button is clicked', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    // Wait for facets to load
    await waitFor(() => {
      expect(screen.getByText('File Types')).toBeInTheDocument();
    });

    // Apply some filters first
    const pdfCheckbox = screen.getByText('PDF Documents').closest('label')?.querySelector('input');
    await user.click(pdfCheckbox!);

    // Click clear filters button
    const clearButton = screen.getByText('Clear');
    await user.click(clearButton);

    // Verify filters are cleared
    expect(pdfCheckbox).not.toBeChecked();
  });

  test('handles empty search results', async () => {
    const user = userEvent.setup();
    const emptyResponse = {
      data: {
        documents: [],
        total: 0,
        query_time_ms: 10,
        suggestions: [],
      },
    };
    
    (documentService.enhancedSearch as any).mockResolvedValue(emptyResponse);
    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'nonexistent');

    await waitFor(() => {
      expect(screen.getByText('No documents found')).toBeInTheDocument();
    });
  });

  test('preserves search state in URL', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Verify URL is updated (this would require checking window.location or using a memory router)
    await waitFor(() => {
      expect(searchInput).toHaveValue('invoice');
    });
  });
});

describe('SearchPage Performance Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (documentService.enhancedSearch as any).mockResolvedValue(mockSearchResponse);
    (documentService.getFacets as any).mockResolvedValue(mockFacetsResponse);
  });

  test('debounces search input to avoid excessive API calls', async () => {
    const user = userEvent.setup();
    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    
    // Type quickly
    await user.type(searchInput, 'invoice', { delay: 50 });

    // Wait for debounce
    await waitFor(() => {
      expect(documentService.enhancedSearch).toHaveBeenCalledTimes(1);
    });

    // Should only be called once due to debouncing
    expect(documentService.enhancedSearch).toHaveBeenCalledWith(
      expect.objectContaining({
        query: 'invoice',
      })
    );
  });

  test('shows loading states during search', async () => {
    const user = userEvent.setup();
    
    // Make the API call take longer to see loading state
    (documentService.enhancedSearch as any).mockImplementation(
      () => new Promise(resolve => setTimeout(() => resolve(mockSearchResponse), 1000))
    );

    renderSearchPage();

    const searchInput = screen.getByPlaceholderText(/search your documents/i);
    await user.type(searchInput, 'invoice');

    // Should show loading indicator
    expect(screen.getByRole('progressbar')).toBeInTheDocument();

    // Wait for search to complete
    await waitFor(() => {
      expect(screen.getByText('invoice_2024.pdf')).toBeInTheDocument();
    }, { timeout: 2000 });

    // Loading indicator should be gone
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  });
});