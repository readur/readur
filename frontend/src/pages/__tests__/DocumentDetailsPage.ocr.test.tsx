import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import DocumentDetailsPage from '../DocumentDetailsPage';
import { ThemeProvider as CustomThemeProvider } from '../../contexts/ThemeContext';
import type { Document, OcrResponse } from '../../services/api';
import * as apiModule from '../../services/api';

const theme = createTheme();

// Mock child components that are not part of the new tab system
vi.mock('../../components/DocumentViewer', () => ({
  default: () => <div data-testid="document-viewer">Document Viewer</div>,
}));

vi.mock('../../components/Labels/LabelSelector', () => ({
  default: () => null,
}));

vi.mock('../../components/RetryHistoryModal', () => ({
  RetryHistoryModal: () => null,
}));

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'documentDetails.ocr.title': 'OCR Text Content',
        'documentDetails.ocr.confidence': 'Confidence',
        'documentDetails.ocr.words': 'Words',
        'documentDetails.ocr.processingTime': 'Processing Time',
        'documentDetails.ocr.loading': 'Loading OCR analysis...',
        'documentDetails.ocr.noText': 'No OCR text available for this document.',
        'documentDetails.ocr.loadFailed': 'OCR text is available but failed to load.',
        'documentDetails.actions.backToDocuments': 'Back to Documents',
        'documentDetails.actions.download': 'Download',
        'documentDetails.actions.deleteDocument': 'Delete Document',
        'documentDetails.actions.viewProcessedImage': 'View Processed Image',
        'documentDetails.tabs.preview': 'Preview',
        'documentDetails.tabs.ocrText': 'OCR Text',
        'documentDetails.tabs.details': 'Details',
        'documentDetails.tabs.activity': 'Activity',
        'documentDetails.dialogs.ocrExpanded.searchPlaceholder': 'Search within extracted text...',
        'documentDetails.errors.notFound': 'Document not found',
      };
      return translations[key] || key;
    },
    i18n: {
      changeLanguage: vi.fn(),
    },
  }),
}));

/**
 * Helper function to create a base mock document
 */
const createBaseMockDocument = (overrides: Partial<Document> = {}): Document => ({
  id: 'test-doc-id',
  filename: 'test.pdf',
  original_filename: 'test.pdf',
  file_path: '/path/to/test.pdf',
  file_size: 1024000,
  mime_type: 'application/pdf',
  tags: [],
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  user_id: 'user-123',
  username: 'testuser',
  has_ocr_text: true,
  ...overrides,
});

/**
 * Helper function to create mock OCR response data
 */
const createMockOcrResponse = (overrides: Partial<OcrResponse> = {}): OcrResponse => ({
  document_id: 'test-doc-id',
  filename: 'test.pdf',
  has_ocr_text: true,
  ocr_text: 'Sample OCR text content',
  ocr_confidence: 95.5,
  ocr_word_count: 290,
  ocr_processing_time_ms: 1500,
  ocr_status: 'completed',
  ocr_completed_at: '2024-01-01T00:01:00Z',
  ...overrides,
});

/**
 * Helper to render DocumentDetailsPage with all necessary providers
 */
const renderDocumentDetailsPage = (documentId = 'test-doc-id') => {
  return render(
    <CustomThemeProvider>
      <ThemeProvider theme={theme}>
        <MemoryRouter initialEntries={[`/documents/${documentId}`]}>
          <Routes>
            <Route path="/documents/:id" element={<DocumentDetailsPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>
    </CustomThemeProvider>
  );
};

/**
 * Helper to navigate to OCR Text tab
 */
const switchToOcrTab = () => {
  const ocrTab = screen.getByRole('tab', { name: 'OCR Text' });
  fireEvent.click(ocrTab);
};

describe('DocumentDetailsPage - OCR Word Count Display', () => {
  beforeEach(() => {
    // Mock window.matchMedia (needed for ThemeContext)
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    // Mock the api.get method for labels
    vi.spyOn(apiModule.default, 'get').mockResolvedValue({
      status: 200,
      data: [],
    } as any);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Test Case 1: Verify OCR word count of 0 renders correctly
   * The component should display "0 words" in the metadata line when ocr_word_count is 0
   */
  test('displays OCR word count of 0 correctly', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
      ocr_word_count: 0,
    });

    const mockOcrData = createMockOcrResponse({
      ocr_word_count: 0,
      ocr_text: '', // Empty document
    });

    vi.spyOn(apiModule.documentService, 'getById').mockResolvedValue({ data: mockDocument } as any);
    vi.spyOn(apiModule.documentService, 'getOcrText').mockResolvedValue({ data: mockOcrData } as any);

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'test.pdf' })).toBeInTheDocument();
    }, { timeout: 3000 });

    // OCR Text is the default tab — verify "0 words" is in the metadata line
    await waitFor(() => {
      expect(screen.getByText(/0 words/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  /**
   * Test Case 2: Verify OCR word count of null does not render
   * When ocr_word_count is null, the word count should not appear in metadata
   */
  test('does not display word count when ocr_word_count is null', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
      ocr_word_count: undefined,
    });

    const mockOcrData = createMockOcrResponse({
      ocr_word_count: null as any, // Explicitly null
    });

    vi.spyOn(apiModule.documentService, 'getById').mockResolvedValue({ data: mockDocument } as any);
    vi.spyOn(apiModule.documentService, 'getOcrText').mockResolvedValue({ data: mockOcrData } as any);

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'test.pdf' })).toBeInTheDocument();
    }, { timeout: 3000 });

    // OCR Text is the default tab — wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Word count should NOT appear in metadata line
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  /**
   * Test Case 3: Verify OCR word count of undefined does not render
   * When ocr_word_count is undefined (field not present), it should not appear
   */
  test('does not display word count when ocr_word_count is undefined', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
    });

    // Explicitly create OCR data without ocr_word_count field
    const mockOcrData: OcrResponse = {
      document_id: 'test-doc-id',
      filename: 'test.pdf',
      has_ocr_text: true,
      ocr_text: 'Some text without word count',
      ocr_confidence: 85.0,
      ocr_processing_time_ms: 1200,
      ocr_status: 'completed',
      // ocr_word_count is intentionally omitted (undefined)
    };

    vi.spyOn(apiModule.documentService, 'getById').mockResolvedValue({ data: mockDocument } as any);
    vi.spyOn(apiModule.documentService, 'getOcrText').mockResolvedValue({ data: mockOcrData } as any);

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'test.pdf' })).toBeInTheDocument();
    }, { timeout: 3000 });

    // OCR Text is the default tab — wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Word count should NOT appear
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  /**
   * Test Case 4: Verify valid OCR word count renders correctly
   * A normal document with a valid word count should display in the metadata line
   */
  test('displays valid OCR word count correctly', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
      ocr_word_count: 290,
    });

    const mockOcrData = createMockOcrResponse({
      ocr_word_count: 290,
      ocr_text: 'This is a sample document with approximately 290 words...',
      ocr_confidence: 95.5,
      ocr_processing_time_ms: 1500,
    });

    vi.spyOn(apiModule.documentService, 'getById').mockResolvedValue({ data: mockDocument } as any);
    vi.spyOn(apiModule.documentService, 'getOcrText').mockResolvedValue({ data: mockOcrData } as any);

    renderDocumentDetailsPage();

    // Wait for the document to load and OCR data to be fetched
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'test.pdf' })).toBeInTheDocument();
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // OCR Text is the default tab — verify metadata line
    // Format: "96% confidence · 290 words · 1500ms · Completed Jan 1"
    await waitFor(() => {
      const metadataText = screen.getByText(/confidence/);
      expect(metadataText.textContent).toMatch(/96% confidence/);
      expect(metadataText.textContent).toMatch(/290 words/);
      expect(metadataText.textContent).toMatch(/1500ms/);
    }, { timeout: 3000 });
  });
});
