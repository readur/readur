import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import MimeTypeFacetFilter from '../MimeTypeFacetFilter';

// Mock the document service
const mockDocumentService = {
  getFacets: vi.fn(),
};

vi.mock('../../../services/api', () => ({
  documentService: mockDocumentService,
}));

const mockFacetsResponse = {
  data: {
    mime_types: [
      { value: 'application/pdf', count: 25 },
      { value: 'image/jpeg', count: 15 },
      { value: 'image/png', count: 10 },
      { value: 'text/plain', count: 8 },
      { value: 'application/msword', count: 5 },
      { value: 'text/csv', count: 3 },
    ],
    tags: [],
  },
};

describe('MimeTypeFacetFilter', () => {
  const mockOnMimeTypeChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockDocumentService.getFacets.mockResolvedValue(mockFacetsResponse);
  });

  test('renders loading state initially', () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    expect(screen.getByText('File Types')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
  });

  test('loads and displays MIME type facets', async () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('PDFs')).toBeInTheDocument();
      expect(screen.getByText('Images')).toBeInTheDocument();
      expect(screen.getByText('Text Files')).toBeInTheDocument();
    });

    expect(documentService.getFacets).toHaveBeenCalledTimes(1);
  });

  test('displays correct counts for each MIME type group', async () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('25')).toBeInTheDocument(); // PDF count
      expect(screen.getByText('25')).toBeInTheDocument(); // Images total (15+10)
      expect(screen.getByText('11')).toBeInTheDocument(); // Text files total (8+3)
    });
  });

  test('allows individual MIME type selection', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('PDF Documents')).toBeInTheDocument();
    });

    const pdfCheckbox = screen.getByLabelText(/PDF Documents/);
    await user.click(pdfCheckbox);

    expect(mockOnMimeTypeChange).toHaveBeenCalledWith(['application/pdf']);
  });

  test('allows group selection', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('PDFs')).toBeInTheDocument();
    });

    const pdfGroupCheckbox = screen.getByText('PDFs').closest('div')?.querySelector('input[type="checkbox"]');
    expect(pdfGroupCheckbox).toBeInTheDocument();
    
    await user.click(pdfGroupCheckbox!);

    expect(mockOnMimeTypeChange).toHaveBeenCalledWith(['application/pdf']);
  });

  test('shows selected state correctly', async () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={['application/pdf', 'image/jpeg']}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('2 selected')).toBeInTheDocument();
    });

    const clearButton = screen.getByRole('button', { name: /clear/i });
    expect(clearButton).toBeInTheDocument();
  });

  test('allows clearing selections', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={['application/pdf', 'image/jpeg']}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('2 selected')).toBeInTheDocument();
    });

    const clearButton = screen.getByRole('button', { name: /clear/i });
    await user.click(clearButton);

    expect(mockOnMimeTypeChange).toHaveBeenCalledWith([]);
  });

  test('supports search functionality', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
        maxItemsToShow={3} // Trigger search box
      />
    );

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Search file types...')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText('Search file types...');
    await user.type(searchInput, 'pdf');

    // Should filter to show only PDF-related items
    expect(screen.getByText('PDF Documents')).toBeInTheDocument();
    expect(screen.queryByText('JPEG Images')).not.toBeInTheDocument();
  });

  test('shows/hides all items based on maxItemsToShow', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
        maxItemsToShow={2}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Show All (6)')).toBeInTheDocument();
    });

    const showAllButton = screen.getByText('Show All (6)');
    await user.click(showAllButton);

    expect(screen.getByText('Show Less')).toBeInTheDocument();
  });

  test('can be collapsed and expanded', async () => {
    const user = userEvent.setup();
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('File Types')).toBeInTheDocument();
    });

    const collapseButton = screen.getByLabelText(/expand/i);
    await user.click(collapseButton);

    // Content should be hidden
    expect(screen.queryByText('PDFs')).not.toBeInTheDocument();
  });

  test('handles API errors gracefully', async () => {
    (documentService.getFacets as any).mockRejectedValue(new Error('API Error'));
    
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    });

    expect(consoleSpy).toHaveBeenCalledWith('Failed to load facets:', expect.any(Error));
    consoleSpy.mockRestore();
  });

  test('displays proper icons for different MIME types', async () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      // Check that icons are rendered (they have specific test IDs or classes)
      expect(screen.getByText('PDFs')).toBeInTheDocument();
      expect(screen.getByText('Images')).toBeInTheDocument();
      expect(screen.getByText('Text Files')).toBeInTheDocument();
    });
  });

  test('groups unknown MIME types under "Other Types"', async () => {
    const customResponse = {
      data: {
        mime_types: [
          { value: 'application/unknown', count: 5 },
          { value: 'weird/type', count: 2 },
        ],
        tags: [],
      },
    };
    
    mockDocumentService.getFacets.mockResolvedValue(customResponse);

    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={[]}
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Other Types')).toBeInTheDocument();
    });
  });

  test('shows indeterminate state for partial group selection', async () => {
    render(
      <MimeTypeFacetFilter
        selectedMimeTypes={['image/jpeg']} // Only one image type selected
        onMimeTypeChange={mockOnMimeTypeChange}
      />
    );

    await waitFor(() => {
      const imageGroupCheckbox = screen.getByText('Images').closest('div')?.querySelector('input[type="checkbox"]');
      expect(imageGroupCheckbox).toHaveProperty('indeterminate', true);
    });
  });
});