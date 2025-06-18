import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import SearchPage from '../SearchPage';

// Mock the document service
const mockDocumentService = {
  search: vi.fn().mockResolvedValue({ data: { documents: [], total: 0 } }),
  enhancedSearch: vi.fn().mockResolvedValue({ data: { documents: [], total: 0 } }),
  getFacets: vi.fn().mockResolvedValue({ data: { mime_types: [], tags: [] } }),
  download: vi.fn(),
};

vi.mock('../../services/api', () => ({
  documentService: mockDocumentService,
}));

// Mock the complex components that might be causing issues
vi.mock('../../components/SearchGuidance', () => ({
  default: () => <div data-testid="search-guidance">Search Guidance</div>,
}));

vi.mock('../../components/EnhancedSearchGuide', () => ({
  default: () => <div data-testid="enhanced-search-guide">Enhanced Search Guide</div>,
}));

vi.mock('../../components/MimeTypeFacetFilter', () => ({
  default: () => <div data-testid="mime-type-facet-filter">File Types</div>,
}));

vi.mock('../../components/EnhancedSnippetViewer', () => ({
  default: () => <div data-testid="enhanced-snippet-viewer">Snippet Viewer</div>,
}));

vi.mock('../../components/AdvancedSearchPanel', () => ({
  default: () => <div data-testid="advanced-search-panel">Advanced Search Panel</div>,
}));

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
  });

  test('renders search page without crashing', () => {
    renderSearchPage();
    expect(screen.getByText('Search Documents')).toBeInTheDocument();
  });

  test('contains search input field', () => {
    renderSearchPage();
    expect(screen.getByPlaceholderText(/search documents/i)).toBeInTheDocument();
  });

  test('shows basic interface elements', () => {
    renderSearchPage();
    
    // Check for main heading
    expect(screen.getByText('Search Documents')).toBeInTheDocument();
    
    // Check for search input
    const searchInput = screen.getByPlaceholderText(/search documents/i);
    expect(searchInput).toBeInTheDocument();
  });
});

describe('SearchPage Performance Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders quickly', () => {
    const startTime = performance.now();
    renderSearchPage();
    const endTime = performance.now();
    
    expect(endTime - startTime).toBeLessThan(1000); // Should render in less than 1 second
    expect(screen.getByText('Search Documents')).toBeInTheDocument();
  });
});