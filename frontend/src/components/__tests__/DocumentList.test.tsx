import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import DocumentList from '../DocumentList';
import type { Document } from '../../services/api';

// Mock the documentService to prevent actual download attempts
vi.mock('../../services/api', () => ({
  documentService: {
    download: vi.fn().mockResolvedValue({ data: new Blob() })
  }
}));

// Mock window.URL methods for download functionality
global.URL.createObjectURL = vi.fn(() => 'mock-object-url');
global.URL.revokeObjectURL = vi.fn();

describe('DocumentList - OCR Metrics Display', () => {
  /**
   * Helper function to create a mock document with sensible defaults
   * All OCR-related fields can be overridden via the overrides parameter
   */
  const createMockDocument = (overrides: Partial<Document> = {}): Document => ({
    id: 'test-id-1',
    user_id: 'user-123',
    filename: 'test-document.pdf',
    original_filename: 'test-document.pdf',
    file_path: '/documents/test-document.pdf',
    mime_type: 'application/pdf',
    file_size: 1024000, // 1MB
    tags: [],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
    has_ocr_text: true,
    ...overrides,
  });

  /**
   * Test Case 1: Document with 0 word count shows "0 words"
   *
   * This is the primary bug fix test case. Previously, when ocr_word_count was 0,
   * the condition `!document.ocr_word_count` evaluated to true (since 0 is falsy),
   * causing the function to return null and display nothing instead of "0 words".
   *
   * After the fix, we now explicitly check `document.ocr_word_count == null`,
   * which correctly allows 0 to pass through and be displayed.
   */
  it('should display "0 words" when ocr_word_count is 0', () => {
    const document = createMockDocument({
      ocr_word_count: 0,
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that "0 words" is rendered in the document list
    expect(screen.getByText(/0 words/i)).toBeInTheDocument();
  });

  /**
   * Test Case 2: Document with null word count shows no metrics
   *
   * When ocr_word_count is explicitly null, it indicates that OCR word counting
   * has not been performed or is unavailable. In this case, no OCR metrics
   * should be displayed.
   */
  it('should not display OCR metrics when ocr_word_count is null', () => {
    const document = createMockDocument({
      ocr_word_count: null,
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that word count is not rendered
    expect(screen.queryByText(/words/i)).not.toBeInTheDocument();
  });

  /**
   * Test Case 3: Document with undefined word count shows no metrics
   *
   * When ocr_word_count is undefined, it indicates the field was not provided.
   * This should behave the same as null - no OCR metrics displayed.
   * The == null check handles both null and undefined.
   */
  it('should not display OCR metrics when ocr_word_count is undefined', () => {
    const document = createMockDocument({
      ocr_word_count: undefined,
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that word count is not rendered
    expect(screen.queryByText(/words/i)).not.toBeInTheDocument();
  });

  /**
   * Test Case 4: Document with valid word count shows correctly
   *
   * Standard case where OCR has been performed and produced a meaningful
   * word count. This verifies normal operation with typical values.
   */
  it('should display correct word count when ocr_word_count has a valid number', () => {
    const document = createMockDocument({
      ocr_word_count: 290,
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that "290 words" is rendered correctly
    expect(screen.getByText(/290 words/i)).toBeInTheDocument();
  });

  /**
   * Test Case 5: Document without OCR text shows no metrics
   *
   * When has_ocr_text is false, it indicates that OCR has not been performed
   * on this document at all. No OCR metrics should be displayed regardless
   * of what ocr_word_count contains.
   */
  it('should not display OCR metrics when has_ocr_text is false', () => {
    const document = createMockDocument({
      has_ocr_text: false,
      ocr_word_count: 100, // Even with a word count, it shouldn't show
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that word count is not rendered when OCR is not available
    expect(screen.queryByText(/words/i)).not.toBeInTheDocument();
  });

  /**
   * Test Case 6: Document with processing time shows both metrics
   *
   * When both word count and processing time are available, both metrics
   * should be displayed with proper formatting (processing time converted
   * from milliseconds to seconds with 1 decimal place).
   */
  it('should display both word count and processing time when available', () => {
    const document = createMockDocument({
      ocr_word_count: 100,
      ocr_processing_time_ms: 1500, // 1.5 seconds
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that both metrics are rendered
    expect(screen.getByText(/100 words/i)).toBeInTheDocument();
    expect(screen.getByText(/1\.5s/i)).toBeInTheDocument();
  });

  /**
   * Additional Test: Edge case with very large word count
   *
   * Ensures the component handles large numbers correctly without
   * formatting issues or overflow.
   */
  it('should handle large word counts correctly', () => {
    const document = createMockDocument({
      ocr_word_count: 1234567,
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify that large numbers are displayed without formatting
    expect(screen.getByText(/1234567 words/i)).toBeInTheDocument();
  });

  /**
   * Additional Test: Processing time formatting
   *
   * Verifies that processing times are correctly converted from milliseconds
   * to seconds and formatted with one decimal place.
   */
  it('should format processing time correctly in seconds', () => {
    const document = createMockDocument({
      ocr_word_count: 50,
      ocr_processing_time_ms: 234, // Should display as 0.2s
      has_ocr_text: true,
    });

    render(<DocumentList documents={[document]} loading={false} />);

    // Verify processing time is formatted to 1 decimal place
    expect(screen.getByText(/0\.2s/i)).toBeInTheDocument();
  });

  /**
   * Additional Test: Multiple documents with different OCR states
   *
   * Ensures the component correctly handles a list of documents where
   * each document has different OCR metrics states.
   */
  it('should handle multiple documents with different OCR metrics', () => {
    const documents = [
      createMockDocument({
        id: 'doc-1',
        original_filename: 'document1.pdf',
        ocr_word_count: 0,
        has_ocr_text: true,
      }),
      createMockDocument({
        id: 'doc-2',
        original_filename: 'document2.pdf',
        ocr_word_count: 500,
        has_ocr_text: true,
      }),
      createMockDocument({
        id: 'doc-3',
        original_filename: 'document3.pdf',
        ocr_word_count: null,
        has_ocr_text: true,
      }),
      createMockDocument({
        id: 'doc-4',
        original_filename: 'document4.pdf',
        has_ocr_text: false,
      }),
    ];

    const { container } = render(<DocumentList documents={documents} loading={false} />);

    // Get all text content from the rendered component
    const renderedText = container.textContent || '';

    // Verify that both "0 words" and "500 words" appear in the rendered output
    expect(renderedText).toContain('0 words'); // doc-1 shows 0 words
    expect(renderedText).toContain('500 words'); // doc-2 shows 500 words

    // Count how many times "words" appears in the rendered text
    // Should be exactly 2 (for doc-1 and doc-2)
    const wordMatches = renderedText.match(/\d+ words/g);
    expect(wordMatches).toHaveLength(2);

    // Verify all document filenames are rendered
    expect(screen.getByText('document1.pdf')).toBeInTheDocument();
    expect(screen.getByText('document2.pdf')).toBeInTheDocument();
    expect(screen.getByText('document3.pdf')).toBeInTheDocument();
    expect(screen.getByText('document4.pdf')).toBeInTheDocument();
  });

  /**
   * Additional Test: Loading state
   *
   * Verifies that the loading state is properly displayed when
   * documents are being fetched.
   */
  it('should display loading state when loading is true', () => {
    render(<DocumentList documents={[]} loading={true} />);

    expect(screen.getByText(/loading documents/i)).toBeInTheDocument();
  });

  /**
   * Additional Test: Empty state
   *
   * Verifies that the empty state is properly displayed when
   * no documents are available.
   */
  it('should display empty state when no documents are available', () => {
    render(<DocumentList documents={[]} loading={false} />);

    expect(screen.getByText(/no documents found/i)).toBeInTheDocument();
  });
});
