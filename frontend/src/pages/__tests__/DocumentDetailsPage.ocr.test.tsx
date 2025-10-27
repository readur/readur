import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import DocumentDetailsPage from '../DocumentDetailsPage';
import { ThemeProvider as CustomThemeProvider } from '../../contexts/ThemeContext';
import type { Document, OcrResponse } from '../../services/api';
import * as apiModule from '../../services/api';

const theme = createTheme();

// Mock all the child components to simplify rendering
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
    t: (key: string) => {
      const translations: Record<string, string> = {
        'documentDetails.ocr.title': 'OCR Text Content',
        'documentDetails.ocr.confidence': 'Confidence',
        'documentDetails.ocr.words': 'Words',
        'documentDetails.ocr.processingTime': 'Processing Time',
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
   * The component should display "0" when ocr_word_count is 0 (using != null check)
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

    // Mock the document service methods
    vi.spyOn(apiModule.documentService, 'getById').mockResolvedValue({ data: mockDocument } as any);
    vi.spyOn(apiModule.documentService, 'getOcrText').mockResolvedValue({ data: mockOcrData } as any);
    vi.spyOn(apiModule.documentService, 'getThumbnail').mockRejectedValue(new Error('No thumbnail'));

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Verify that the word count section renders with value "0"
    await waitFor(() => {
      expect(screen.getByText('0')).toBeInTheDocument();
      expect(screen.getByText('Words')).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  /**
   * Test Case 2: Verify OCR word count of null does not render
   * When ocr_word_count is null, the word count stat box should not appear
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
    vi.spyOn(apiModule.documentService, 'getThumbnail').mockRejectedValue(new Error('No thumbnail'));

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Wait for component to finish rendering
    await waitFor(() => {
      // The document title should be visible
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Word count stat box should not render - check there's no "Words" label
    const wordsLabels = screen.queryAllByText('Words');
    expect(wordsLabels.length).toBe(0);
  });

  /**
   * Test Case 3: Verify OCR word count of undefined does not render
   * When ocr_word_count is undefined (field not present), the stat box should not appear
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
    vi.spyOn(apiModule.documentService, 'getThumbnail').mockRejectedValue(new Error('No thumbnail'));

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Wait for component to finish rendering
    await waitFor(() => {
      // The document title should be visible
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Word count should NOT render - no "Words" label
    const wordsLabels = screen.queryAllByText('Words');
    expect(wordsLabels.length).toBe(0);
  });

  /**
   * Test Case 4: Verify valid OCR word count renders correctly
   * A normal document with a valid word count should display properly
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
    vi.spyOn(apiModule.documentService, 'getThumbnail').mockRejectedValue(new Error('No thumbnail'));

    renderDocumentDetailsPage();

    // Wait for the document to load
    await waitFor(() => {
      expect(screen.getByText('test.pdf')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Wait for OCR data to load
    await waitFor(() => {
      expect(apiModule.documentService.getOcrText).toHaveBeenCalled();
    }, { timeout: 3000 });

    // Verify word count displays with proper formatting
    await waitFor(() => {
      expect(screen.getByText('290')).toBeInTheDocument();
      expect(screen.getByText('Words')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Also verify confidence is displayed (95.5 rounds to 96)
    await waitFor(() => {
      expect(screen.getByText(/96%/)).toBeInTheDocument();
      expect(screen.getByText('Confidence')).toBeInTheDocument();
    }, { timeout: 3000 });

    // Verify processing time is displayed
    await waitFor(() => {
      expect(screen.getByText('1500ms')).toBeInTheDocument();
      expect(screen.getByText('Processing Time')).toBeInTheDocument();
    }, { timeout: 3000 });
  });
});
