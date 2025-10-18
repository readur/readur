import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';

// Mock the entire api module with mock functions
vi.mock('../../services/api', async () => {
  const actual = await vi.importActual<typeof import('../../services/api')>('../../services/api');
  return {
    ...actual,
    documentService: {
      getById: vi.fn(),
      download: vi.fn(),
      getOcrText: vi.fn(),
      getThumbnail: vi.fn(),
      getProcessedImage: vi.fn(),
      bulkRetryOcr: vi.fn(),
      delete: vi.fn(),
    },
    default: {
      get: vi.fn(),
      post: vi.fn(),
      put: vi.fn(),
      delete: vi.fn(),
    },
  };
});

// Mock components that are used by DocumentDetailsPage but not part of our test focus
vi.mock('../../components/DocumentViewer', () => ({
  default: () => null,
}));

vi.mock('../../components/Labels/LabelSelector', () => ({
  default: () => null,
}));

vi.mock('../../components/MetadataDisplay', () => ({
  default: () => null,
}));

vi.mock('../../components/FileIntegrityDisplay', () => ({
  default: () => null,
}));

vi.mock('../../components/ProcessingTimeline', () => ({
  default: () => null,
}));

vi.mock('../../components/RetryHistoryModal', () => ({
  RetryHistoryModal: () => null,
}));

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, params?: any) => {
      // Provide simple translations for the keys we need
      const translations: Record<string, string> = {
        'documentDetails.errors.notFound': 'Document not found',
        'documentDetails.actions.backToDocuments': 'Back to Documents',
        'documentDetails.actions.download': 'Download',
        'documentDetails.actions.viewDocument': 'View Document',
        'documentDetails.actions.viewOcrText': 'View OCR Text',
        'documentDetails.actions.deleteDocument': 'Delete Document',
        'documentDetails.actions.editLabels': 'Edit Labels',
        'documentDetails.actions.viewProcessedImage': 'View Processed Image',
        'documentDetails.actions.retryOcr': 'Retry OCR',
        'documentDetails.actions.retryHistory': 'Retry History',
        'documentDetails.subtitle': 'Document Details',
        'documentDetails.metadata.fileSize': 'File Size',
        'documentDetails.metadata.uploadDate': 'Upload Date',
        'documentDetails.metadata.sourceType': 'Source Type',
        'documentDetails.metadata.originalPath': 'Original Path',
        'documentDetails.metadata.originalCreated': 'Original Created',
        'documentDetails.metadata.originalModified': 'Original Modified',
        'documentDetails.metadata.ocrStatus': 'OCR Status',
        'documentDetails.metadata.textExtracted': 'Text Extracted',
        'documentDetails.ocr.title': 'OCR Text Content',
        'documentDetails.ocr.confidence': 'Confidence',
        'documentDetails.ocr.words': 'Words',
        'documentDetails.ocr.processingTime': 'Processing Time',
        'documentDetails.ocr.loading': 'Loading OCR text...',
        'documentDetails.ocr.loadFailed': 'Failed to load OCR text',
        'documentDetails.ocr.noText': 'No OCR text available',
        'documentDetails.ocr.error': 'OCR Error',
        'documentDetails.ocr.expand': 'Expand',
        'documentDetails.ocr.expandTooltip': 'Expand OCR Text',
        'documentDetails.tagsLabels.title': 'Tags & Labels',
        'documentDetails.tagsLabels.tags': 'Tags',
        'documentDetails.tagsLabels.labels': 'Labels',
        'documentDetails.tagsLabels.noLabels': 'No labels assigned',
        'navigation.documents': 'Documents',
        'common.status.error': 'An error occurred',
        'common.actions.close': 'Close',
        'common.actions.download': 'Download',
        'common.actions.cancel': 'Cancel',
      };

      if (params) {
        let translation = translations[key] || key;
        // Simple parameter replacement
        Object.keys(params).forEach((param) => {
          translation = translation.replace(`{{${param}}}`, params[param]);
        });
        return translation;
      }

      return translations[key] || key;
    },
    i18n: {
      changeLanguage: vi.fn(),
    },
  }),
}));

// Import components and types AFTER the mocks are set up
import DocumentDetailsPage from '../DocumentDetailsPage';
import * as apiModule from '../../services/api';
import type { Document, OcrResponse } from '../../services/api';
import { ThemeProvider as CustomThemeProvider } from '../../contexts/ThemeContext';

// Get references to the mocked services
const mockDocumentService = vi.mocked(apiModule.documentService, true);
const mockApi = vi.mocked(apiModule.default, true);

// Create MUI theme for wrapping components
const theme = createTheme();

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

describe('DocumentDetailsPage - OCR Word Count Display', () => {
  beforeEach(() => {
    console.log('mockDocumentService:', mockDocumentService);
    console.log('mockDocumentService.getThumbnail:', mockDocumentService.getThumbnail);
    vi.clearAllMocks();

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

    // Setup all default mocks - use type assertion since we know they're vi.fn() mocks
    (mockDocumentService.getThumbnail as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('No thumbnail'));
    (mockDocumentService.bulkRetryOcr as ReturnType<typeof vi.fn>).mockResolvedValue({ data: { success: true } } as any);
    (mockDocumentService.delete as ReturnType<typeof vi.fn>).mockResolvedValue({} as any);
    (mockApi.get as ReturnType<typeof vi.fn>).mockResolvedValue({ status: 200, data: [] });
    (mockApi.post as ReturnType<typeof vi.fn>).mockResolvedValue({ status: 200, data: {} });
    (mockApi.put as ReturnType<typeof vi.fn>).mockResolvedValue({ status: 200, data: {} });
  });

  /**
   * Test Case 1: Verify OCR word count of 0 renders correctly
   *
   * This tests the bug fix at lines 839, 1086, and 1184 where we changed:
   * - Before: {ocrData.ocr_word_count && (
   * - After: {ocrData.ocr_word_count != null && (
   *
   * With ocr_word_count = 0, the old condition would be falsy and not render,
   * but the new condition correctly checks for null/undefined.
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

    (mockDocumentService.getById as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockDocument });
    (mockDocumentService.getOcrText as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockOcrData });

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(mockDocumentService.getOcrText).toHaveBeenCalled();
    });

    // Verify that the word count section renders (it should now with != null check)
    await waitFor(() => {
      // The word count should be displayed as "0"
      const wordCountElements = screen.getAllByText('0');
      expect(wordCountElements.length).toBeGreaterThan(0);

      // Verify "Words" label is present (indicates the stat box rendered)
      expect(screen.getByText('Words')).toBeInTheDocument();
    });
  });

  /**
   * Test Case 2: Verify OCR word count of null does not render
   *
   * When ocr_word_count is null, the != null check should be false,
   * and the word count stat should not appear.
   */
  test('does not display word count when ocr_word_count is null', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
      ocr_word_count: undefined, // Will be null in the API response
    });

    const mockOcrData = createMockOcrResponse({
      ocr_word_count: undefined,
    });

    (mockDocumentService.getById as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockDocument });
    (mockDocumentService.getOcrText as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockOcrData });

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(mockDocumentService.getOcrText).toHaveBeenCalled();
    });

    // Verify OCR section still renders (document has OCR text)
    await waitFor(() => {
      expect(screen.getByText('OCR Text Content')).toBeInTheDocument();
    });

    // Word count stat box should not render
    // We check that "Words" label doesn't appear in the stats section
    const wordsLabels = screen.queryAllByText('Words');
    expect(wordsLabels.length).toBe(0);
  });

  /**
   * Test Case 3: Verify OCR word count of undefined does not render
   *
   * Similar to null case - when the field is explicitly undefined,
   * the stat should not render.
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
      ocr_text: 'Some text',
      ocr_confidence: 85.0,
      ocr_processing_time_ms: 1200,
      ocr_status: 'completed',
      // ocr_word_count is intentionally omitted
    };

    (mockDocumentService.getById as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockDocument });
    (mockDocumentService.getOcrText as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockOcrData });

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(mockDocumentService.getOcrText).toHaveBeenCalled();
    });

    // Verify OCR section renders
    await waitFor(() => {
      expect(screen.getByText('OCR Text Content')).toBeInTheDocument();
    });

    // Confidence should render (it's present in mockOcrData)
    await waitFor(() => {
      expect(screen.getByText(/85%/)).toBeInTheDocument();
    });

    // Word count should NOT render
    const wordsLabels = screen.queryAllByText('Words');
    expect(wordsLabels.length).toBe(0);
  });

  /**
   * Test Case 4: Verify valid OCR word count renders correctly
   *
   * This is the happy path - a normal document with a valid word count
   * should display properly.
   */
  test('displays valid OCR word count correctly', async () => {
    const mockDocument = createBaseMockDocument({
      has_ocr_text: true,
      ocr_word_count: 290,
    });

    const mockOcrData = createMockOcrResponse({
      ocr_word_count: 290,
      ocr_text: 'This is a sample document with approximately 290 words...',
    });

    (mockDocumentService.getById as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockDocument });
    (mockDocumentService.getOcrText as ReturnType<typeof vi.fn>).mockResolvedValue({ data: mockOcrData });

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(mockDocumentService.getOcrText).toHaveBeenCalled();
    });

    // Verify word count displays with proper formatting
    await waitFor(() => {
      // Should display "290" formatted with toLocaleString()
      expect(screen.getByText('290')).toBeInTheDocument();
      expect(screen.getByText('Words')).toBeInTheDocument();
    });

    // Also verify confidence is displayed
    await waitFor(() => {
      expect(screen.getByText(/96%/)).toBeInTheDocument(); // 95.5 rounds to 96
      expect(screen.getByText('Confidence')).toBeInTheDocument();
    });

    // Verify processing time is displayed
    await waitFor(() => {
      expect(screen.getByText('1500ms')).toBeInTheDocument();
      expect(screen.getByText('Processing Time')).toBeInTheDocument();
    });
  });
});
