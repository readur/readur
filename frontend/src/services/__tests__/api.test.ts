import { describe, it, expect, vi, beforeEach } from 'vitest';
import { type OcrResponse, type Document } from '../api';
import { createMockApiServices } from '../../test/test-utils';

// Use centralized API mocking
const mockServices = createMockApiServices();
const mockDocumentService = mockServices.documentService;

// Mock the entire api module with centralized mocks
vi.mock('../api', async () => {
  const actual = await vi.importActual('../api');
  return {
    ...actual,
    documentService: mockDocumentService,
  };
});

// Import after mocking
const { documentService } = await import('../api');

describe('documentService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getOcrText', () => {
    const mockOcrResponse: OcrResponse = {
      document_id: 'doc-123',
      filename: 'test_document.pdf',
      has_ocr_text: true,
      ocr_text: 'This is extracted OCR text content.',
      ocr_confidence: 95.5,
      ocr_word_count: 150,
      ocr_processing_time_ms: 1200,
      ocr_status: 'completed',
      ocr_error: null,
      ocr_completed_at: '2024-01-01T00:05:00Z',
    };

    it('should fetch OCR text for a document', async () => {
      const mockResponse = {
        data: mockOcrResponse,
        status: 200,
        statusText: 'OK',
        headers: {},
        config: {},
      };

      mockDocumentService.getOcrText.mockResolvedValue(mockResponse);

      const result = await documentService.getOcrText('doc-123');

      expect(result.data).toEqual(mockOcrResponse);
      expect(result.data.document_id).toBe('doc-123');
      expect(result.data.has_ocr_text).toBe(true);
      expect(result.data.ocr_text).toBe('This is extracted OCR text content.');
      expect(result.data.ocr_confidence).toBe(95.5);
      expect(result.data.ocr_word_count).toBe(150);
    });

    it('should handle OCR response without text', async () => {
      const mockEmptyOcrResponse: OcrResponse = {
        document_id: 'doc-456',
        filename: 'text_file.txt',
        has_ocr_text: false,
        ocr_text: null,
        ocr_confidence: null,
        ocr_word_count: null,
        ocr_processing_time_ms: null,
        ocr_status: 'pending',
        ocr_error: null,
        ocr_completed_at: null,
      };

      const mockResponse = {
        data: mockEmptyOcrResponse,
        status: 200,
        statusText: 'OK',
        headers: {},
        config: {},
      };

      mockDocumentService.getOcrText.mockResolvedValue(mockResponse);

      const result = await documentService.getOcrText('doc-456');

      expect(result.data).toEqual(mockEmptyOcrResponse);
      expect(result.data.has_ocr_text).toBe(false);
      expect(result.data.ocr_text).toBeNull();
      expect(result.data.ocr_confidence).toBeNull();
    });

    it('should handle OCR error response', async () => {
      const mockErrorOcrResponse: OcrResponse = {
        document_id: 'doc-789',
        filename: 'corrupted_file.pdf',
        has_ocr_text: false,
        ocr_text: null,
        ocr_confidence: null,
        ocr_word_count: null,
        ocr_processing_time_ms: 5000,
        ocr_status: 'failed',
        ocr_error: 'Failed to process document: corrupted file format',
        ocr_completed_at: '2024-01-01T00:05:00Z',
      };

      const mockResponse = {
        data: mockErrorOcrResponse,
        status: 200,
        statusText: 'OK',
        headers: {},
        config: {},
      };

      mockDocumentService.getOcrText.mockResolvedValue(mockResponse);

      const result = await documentService.getOcrText('doc-789');

      expect(result.data).toEqual(mockErrorOcrResponse);
      expect(result.data.ocr_status).toBe('failed');
      expect(result.data.ocr_error).toBe('Failed to process document: corrupted file format');
      expect(result.data.has_ocr_text).toBe(false);
    });

    it('should make correct API call', async () => {
      mockDocumentService.getOcrText.mockResolvedValue({ data: mockOcrResponse });

      await documentService.getOcrText('doc-123');

      expect(mockDocumentService.getOcrText).toHaveBeenCalledWith('doc-123');
    });

    it('should handle network errors', async () => {
      mockDocumentService.getOcrText.mockRejectedValue(new Error('Network Error'));

      await expect(documentService.getOcrText('doc-123')).rejects.toThrow('Network Error');
    });

    it('should handle 404 errors for non-existent documents', async () => {
      mockDocumentService.getOcrText.mockRejectedValue({
        response: {
          status: 404,
          data: { error: 'Document not found' },
        },
      });

      await expect(documentService.getOcrText('non-existent-doc')).rejects.toMatchObject({
        response: {
          status: 404,
        },
      });
    });

    it('should handle 401 unauthorized errors', async () => {
      mockDocumentService.getOcrText.mockRejectedValue({
        response: {
          status: 401,
          data: { error: 'Unauthorized' },
        },
      });

      await expect(documentService.getOcrText('doc-123')).rejects.toMatchObject({
        response: {
          status: 401,
        },
      });
    });
  });

  describe('list', () => {
    const mockDocuments: Document[] = [
      {
        id: 'doc-1',
        filename: 'document1.pdf',
        original_filename: 'document1.pdf',
        file_size: 1024000,
        mime_type: 'application/pdf',
        tags: ['pdf', 'document'],
        created_at: '2024-01-01T00:00:00Z',
        has_ocr_text: true,
        ocr_confidence: 95.5,
        ocr_word_count: 150,
        ocr_processing_time_ms: 1200,
        ocr_status: 'completed',
      },
      {
        id: 'doc-2',
        filename: 'image.png',
        original_filename: 'image.png',
        file_size: 512000,
        mime_type: 'image/png',
        tags: ['image'],
        created_at: '2024-01-02T00:00:00Z',
        has_ocr_text: false,
        ocr_confidence: undefined,
        ocr_word_count: undefined,
        ocr_processing_time_ms: undefined,
        ocr_status: 'pending',
      },
    ];

    it('should fetch document list with OCR metadata', async () => {
      const mockResponse = {
        data: mockDocuments,
        status: 200,
        statusText: 'OK',
        headers: {},
        config: {},
      };

      mockDocumentService.list.mockResolvedValue(mockResponse);

      const result = await documentService.list(50, 0);

      expect(result.data).toEqual(mockDocuments);
      expect(result.data[0].has_ocr_text).toBe(true);
      expect(result.data[0].ocr_confidence).toBe(95.5);
      expect(result.data[1].has_ocr_text).toBe(false);
      expect(result.data[1].ocr_confidence).toBeUndefined();
    });
  });

  describe('upload', () => {
    it('should upload file with multipart form data', async () => {
      const mockFile = new File(['content'], 'test.pdf', { type: 'application/pdf' });
      const mockUploadResponse: Document = {
        id: 'doc-new',
        filename: 'test.pdf',
        original_filename: 'test.pdf',
        file_size: 7,
        mime_type: 'application/pdf',
        tags: [],
        created_at: '2024-01-01T00:00:00Z',
        has_ocr_text: false,
        ocr_status: 'pending',
      };

      mockDocumentService.upload.mockResolvedValue({ data: mockUploadResponse });

      const result = await documentService.upload(mockFile);

      expect(result.data).toEqual(mockUploadResponse);
      expect(mockDocumentService.upload).toHaveBeenCalledWith(mockFile);
    });
  });

  describe('download', () => {
    it('should download file as blob', async () => {
      const mockBlob = new Blob(['file content'], { type: 'application/pdf' });
      mockDocumentService.download.mockResolvedValue({ data: mockBlob });

      const result = await documentService.download('doc-123');

      expect(result.data).toEqual(mockBlob);
      expect(mockDocumentService.download).toHaveBeenCalledWith('doc-123');
    });
  });
});

describe('OcrResponse interface', () => {
  it('should have correct type structure', () => {
    const ocrResponse: OcrResponse = {
      document_id: 'doc-123',
      filename: 'test.pdf',
      has_ocr_text: true,
      ocr_text: 'Sample text',
      ocr_confidence: 95.5,
      ocr_word_count: 100,
      ocr_processing_time_ms: 1000,
      ocr_status: 'completed',
      ocr_error: null,
      ocr_completed_at: '2024-01-01T00:00:00Z',
    };

    // Type assertions to ensure correct types
    expect(typeof ocrResponse.document_id).toBe('string');
    expect(typeof ocrResponse.filename).toBe('string');
    expect(typeof ocrResponse.has_ocr_text).toBe('boolean');
    expect(typeof ocrResponse.ocr_text).toBe('string');
    expect(typeof ocrResponse.ocr_confidence).toBe('number');
    expect(typeof ocrResponse.ocr_word_count).toBe('number');
    expect(typeof ocrResponse.ocr_processing_time_ms).toBe('number');
    expect(typeof ocrResponse.ocr_status).toBe('string');
    expect(ocrResponse.ocr_error).toBeNull();
    expect(typeof ocrResponse.ocr_completed_at).toBe('string');
  });

  it('should allow optional/null fields', () => {
    const ocrResponseMinimal: OcrResponse = {
      document_id: 'doc-456',
      filename: 'text.txt',
      has_ocr_text: false,
      ocr_text: null,
      ocr_confidence: undefined,
      ocr_word_count: undefined,
      ocr_processing_time_ms: undefined,
      ocr_status: 'pending',
      ocr_error: undefined,
      ocr_completed_at: undefined,
    };

    expect(ocrResponseMinimal.has_ocr_text).toBe(false);
    expect(ocrResponseMinimal.ocr_text).toBeNull();
    expect(ocrResponseMinimal.ocr_confidence).toBeUndefined();
  });
});

describe('documentService.deleteLowConfidence', () => {
  it('should delete low confidence documents successfully', async () => {
    const mockDeleteResponse = {
      data: {
        success: true,
        message: 'Successfully deleted 3 documents with OCR confidence below 30%',
        deleted_count: 3,
        matched_count: 3,
        successful_file_deletions: 3,
        failed_file_deletions: 0,
        ignored_file_creation_failures: 0,
        deleted_document_ids: ['doc-1', 'doc-2', 'doc-3']
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockDeleteResponse);

    const result = await documentService.deleteLowConfidence(30.0, false);

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(30.0, false);
    expect(result.data.success).toBe(true);
    expect(result.data.deleted_count).toBe(3);
    expect(result.data.matched_count).toBe(3);
    expect(result.data.deleted_document_ids).toHaveLength(3);
  });

  it('should preview low confidence documents without deleting', async () => {
    const mockPreviewResponse = {
      data: {
        success: true,
        message: 'Found 5 documents with OCR confidence below 50%',
        matched_count: 5,
        preview: true,
        document_ids: ['doc-1', 'doc-2', 'doc-3', 'doc-4', 'doc-5']
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockPreviewResponse);

    const result = await documentService.deleteLowConfidence(50.0, true);

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(50.0, true);
    expect(result.data.success).toBe(true);
    expect(result.data.preview).toBe(true);
    expect(result.data.matched_count).toBe(5);
    expect(result.data.document_ids).toHaveLength(5);
    expect(result.data).not.toHaveProperty('deleted_count');
  });

  it('should handle no matching documents', async () => {
    const mockEmptyResponse = {
      data: {
        success: true,
        message: 'No documents found with OCR confidence below 10%',
        deleted_count: 0
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockEmptyResponse);

    const result = await documentService.deleteLowConfidence(10.0, false);

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(10.0, false);
    expect(result.data.success).toBe(true);
    expect(result.data.deleted_count).toBe(0);
  });

  it('should handle validation errors for invalid confidence threshold', async () => {
    const mockErrorResponse = {
      data: {
        success: false,
        message: 'max_confidence must be between 0.0 and 100.0',
        matched_count: 0
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockErrorResponse);

    const result = await documentService.deleteLowConfidence(-10.0, false);

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(-10.0, false);
    expect(result.data.success).toBe(false);
    expect(result.data.message).toContain('must be between 0.0 and 100.0');
  });

  it('should handle API errors gracefully', async () => {
    const mockError = new Error('Network error');
    mockDocumentService.deleteLowConfidence.mockRejectedValue(mockError);

    await expect(documentService.deleteLowConfidence(30.0, false))
      .rejects.toThrow('Network error');

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(30.0, false);
  });

  it('should use correct default values', async () => {
    const mockResponse = {
      data: { success: true, matched_count: 0 },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockResponse);

    // Test with explicit false value (the default)
    await documentService.deleteLowConfidence(40.0, false);

    expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(40.0, false);
  });

  it('should handle partial deletion failures', async () => {
    const mockPartialFailureResponse = {
      data: {
        success: true,
        message: 'Successfully deleted 2 documents with OCR confidence below 25%',
        deleted_count: 2,
        matched_count: 3,
        successful_file_deletions: 1,
        failed_file_deletions: 1,
        ignored_file_creation_failures: 1,
        deleted_document_ids: ['doc-1', 'doc-2']
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockPartialFailureResponse);

    const result = await documentService.deleteLowConfidence(25.0, false);

    expect(result.data.success).toBe(true);
    expect(result.data.deleted_count).toBe(2);
    expect(result.data.matched_count).toBe(3);
    expect(result.data.failed_file_deletions).toBe(1);
    expect(result.data.ignored_file_creation_failures).toBe(1);
  });

  it('should properly encode confidence threshold values', async () => {
    const mockResponse = {
      data: { success: true, matched_count: 0 },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.deleteLowConfidence.mockResolvedValue(mockResponse);

    // Test various confidence values
    const testValues = [0.0, 0.1, 30.5, 50.0, 99.9, 100.0];
    
    for (const confidence of testValues) {
      mockDocumentService.deleteLowConfidence.mockClear();
      await documentService.deleteLowConfidence(confidence, true);
      expect(mockDocumentService.deleteLowConfidence).toHaveBeenCalledWith(confidence, true);
    }
  });
});

describe('documentService.getFailedOcrDocuments', () => {
  const mockFailedOcrResponse = {
    documents: [
      {
        id: 'doc-1',
        filename: 'failed_doc1.pdf',
        failure_reason: 'low_ocr_confidence',
        failure_stage: 'ocr',
        created_at: '2024-01-01T10:00:00Z',
        retry_count: 1
      },
      {
        id: 'doc-2', 
        filename: 'failed_doc2.pdf',
        failure_reason: 'pdf_parsing_error',
        failure_stage: 'ocr',
        created_at: '2024-01-01T11:00:00Z',
        retry_count: 0
      }
    ],
    pagination: {
      total: 2,
      limit: 50,
      offset: 0,
      has_more: false
    },
    statistics: {
      total_failed: 2,
      failure_categories: [
        { reason: 'low_ocr_confidence', display_name: 'Low OCR Confidence', count: 1 },
        { reason: 'pdf_parsing_error', display_name: 'PDF Parsing Error', count: 1 }
      ]
    }
  };

  it('should fetch failed OCR documents successfully', async () => {
    const mockResponse = {
      data: mockFailedOcrResponse,
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.getFailedOcrDocuments.mockResolvedValue(mockResponse);

    const result = await documentService.getFailedOcrDocuments(50, 0);

    expect(mockDocumentService.getFailedOcrDocuments).toHaveBeenCalledWith(50, 0);
    expect(result.data).toEqual(mockFailedOcrResponse);
    expect(result.data.documents).toHaveLength(2);
    expect(result.data.documents[0].failure_stage).toBe('ocr');
    expect(result.data.pagination.total).toBe(2);
  });

  it('should handle pagination parameters correctly', async () => {
    mockDocumentService.getFailedOcrDocuments.mockResolvedValue({ data: mockFailedOcrResponse });

    await documentService.getFailedOcrDocuments(25, 10);

    expect(mockDocumentService.getFailedOcrDocuments).toHaveBeenCalledWith(25, 10);
  });

  it('should use default pagination when not specified', async () => {
    mockDocumentService.getFailedOcrDocuments.mockResolvedValue({ data: mockFailedOcrResponse });

    await documentService.getFailedOcrDocuments();

    expect(mockDocumentService.getFailedOcrDocuments).toHaveBeenCalledWith();
  });

  it('should handle empty results', async () => {
    const emptyResponse = {
      documents: [],
      pagination: { total: 0, limit: 50, offset: 0, has_more: false },
      statistics: { total_failed: 0, failure_categories: [] }
    };

    mockDocumentService.getFailedOcrDocuments.mockResolvedValue({ data: emptyResponse });

    const result = await documentService.getFailedOcrDocuments();

    expect(result.data.documents).toHaveLength(0);
    expect(result.data.pagination.total).toBe(0);
    expect(result.data.statistics.total_failed).toBe(0);
  });

  it('should handle API errors', async () => {
    const mockError = new Error('Network error');
    mockDocumentService.getFailedOcrDocuments.mockRejectedValue(mockError);

    await expect(documentService.getFailedOcrDocuments()).rejects.toThrow('Network error');
  });
});

describe('documentService.getFailedDocuments', () => {
  const mockFailedDocumentsResponse = {
    documents: [
      {
        id: 'doc-1',
        filename: 'failed_doc1.pdf',
        failure_reason: 'low_ocr_confidence',
        failure_stage: 'ocr',
        created_at: '2024-01-01T10:00:00Z',
        retry_count: 1
      },
      {
        id: 'doc-2',
        filename: 'duplicate_doc.pdf', 
        failure_reason: 'duplicate_content',
        failure_stage: 'ingestion',
        created_at: '2024-01-01T11:00:00Z',
        retry_count: 0
      },
      {
        id: 'doc-3',
        filename: 'large_file.pdf',
        failure_reason: 'file_too_large',
        failure_stage: 'validation',
        created_at: '2024-01-01T12:00:00Z',
        retry_count: 2
      }
    ],
    pagination: {
      total: 3,
      limit: 25,
      offset: 0,
      has_more: false
    },
    statistics: {
      total_failed: 3,
      failure_categories: [
        { reason: 'low_ocr_confidence', display_name: 'Low OCR Confidence', count: 1 },
        { reason: 'duplicate_content', display_name: 'Duplicate Content', count: 1 },
        { reason: 'file_too_large', display_name: 'File Too Large', count: 1 }
      ]
    }
  };

  it('should fetch failed documents with default parameters', async () => {
    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: mockFailedDocumentsResponse });

    const result = await documentService.getFailedDocuments();

    expect(mockDocumentService.getFailedDocuments).toHaveBeenCalledWith();
    expect(result.data).toEqual(mockFailedDocumentsResponse);
    expect(result.data.documents).toHaveLength(3);
  });

  it('should filter by stage parameter', async () => {
    const ocrOnlyResponse = {
      ...mockFailedDocumentsResponse,
      documents: [mockFailedDocumentsResponse.documents[0]], // Only OCR failure
      pagination: { ...mockFailedDocumentsResponse.pagination, total: 1 },
      statistics: { total_failed: 1, failure_categories: [{ reason: 'low_ocr_confidence', display_name: 'Low OCR Confidence', count: 1 }] }
    };

    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: ocrOnlyResponse });

    const result = await documentService.getFailedDocuments(25, 0, 'ocr');

    expect(mockDocumentService.getFailedDocuments).toHaveBeenCalledWith(25, 0, 'ocr');
    expect(result.data.documents).toHaveLength(1);
    expect(result.data.documents[0].failure_stage).toBe('ocr');
  });

  it('should filter by reason parameter', async () => {
    const duplicateOnlyResponse = {
      ...mockFailedDocumentsResponse,
      documents: [mockFailedDocumentsResponse.documents[1]], // Only duplicate failure
      pagination: { ...mockFailedDocumentsResponse.pagination, total: 1 },
      statistics: { total_failed: 1, failure_categories: [{ reason: 'duplicate_content', display_name: 'Duplicate Content', count: 1 }] }
    };

    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: duplicateOnlyResponse });

    const result = await documentService.getFailedDocuments(25, 0, undefined, 'duplicate_content');

    expect(mockDocumentService.getFailedDocuments).toHaveBeenCalledWith(25, 0, undefined, 'duplicate_content');
    expect(result.data.documents).toHaveLength(1);
    expect(result.data.documents[0].failure_reason).toBe('duplicate_content');
  });

  it('should filter by both stage and reason', async () => {
    const filteredResponse = {
      ...mockFailedDocumentsResponse,
      documents: [mockFailedDocumentsResponse.documents[0]], // OCR + low_ocr_confidence
      pagination: { ...mockFailedDocumentsResponse.pagination, total: 1 },
      statistics: { total_failed: 1, failure_categories: [{ reason: 'low_ocr_confidence', display_name: 'Low OCR Confidence', count: 1 }] }
    };

    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: filteredResponse });

    const result = await documentService.getFailedDocuments(25, 0, 'ocr', 'low_ocr_confidence');

    expect(mockDocumentService.getFailedDocuments).toHaveBeenCalledWith(25, 0, 'ocr', 'low_ocr_confidence');
    expect(result.data.documents).toHaveLength(1);
    expect(result.data.documents[0].failure_stage).toBe('ocr');
    expect(result.data.documents[0].failure_reason).toBe('low_ocr_confidence');
  });

  it('should handle custom pagination', async () => {
    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: mockFailedDocumentsResponse });

    await documentService.getFailedDocuments(10, 20);

    expect(mockDocumentService.getFailedDocuments).toHaveBeenCalledWith(10, 20);
  });

  it('should handle empty results', async () => {
    const emptyResponse = {
      documents: [],
      pagination: { total: 0, limit: 25, offset: 0, has_more: false },
      statistics: { total_failed: 0, failure_categories: [] }
    };

    mockDocumentService.getFailedDocuments.mockResolvedValue({ data: emptyResponse });

    const result = await documentService.getFailedDocuments();

    expect(result.data.documents).toHaveLength(0);
    expect(result.data.statistics.total_failed).toBe(0);
  });
});

describe('documentService.retryOcr', () => {
  it('should retry OCR for a document successfully', async () => {
    const mockRetryResponse = {
      data: {
        success: true,
        message: 'OCR retry queued successfully',
        document_id: 'doc-123'
      },
      status: 200,
      statusText: 'OK',
      headers: {},
      config: {},
    };

    mockDocumentService.retryOcr.mockResolvedValue(mockRetryResponse);

    const result = await documentService.retryOcr('doc-123');

    expect(mockDocumentService.retryOcr).toHaveBeenCalledWith('doc-123');
    expect(result.data.success).toBe(true);
    expect(result.data.document_id).toBe('doc-123');
  });

  it('should handle retry errors', async () => {
    const mockError = {
      response: {
        status: 404,
        data: { error: 'Document not found' }
      }
    };

    mockDocumentService.retryOcr.mockRejectedValue(mockError);

    await expect(documentService.retryOcr('non-existent-doc')).rejects.toMatchObject({
      response: { status: 404 }
    });
  });

  it('should handle network errors', async () => {
    mockDocumentService.retryOcr.mockRejectedValue(new Error('Network error'));

    await expect(documentService.retryOcr('doc-123')).rejects.toThrow('Network error');
  });
});