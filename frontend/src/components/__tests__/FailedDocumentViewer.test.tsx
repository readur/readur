import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import FailedDocumentViewer from '../FailedDocumentViewer';
import { api } from '../../services/api';
import { renderWithProviders, setupTestEnvironment } from '../../test/test-utils';

// Mock the API
vi.mock('../../services/api', () => ({
  api: {
    get: vi.fn(),
  },
}));


const defaultProps = {
  failedDocumentId: 'test-failed-doc-id',
  filename: 'test-document.pdf',
  mimeType: 'application/pdf',
};

const renderFailedDocumentViewer = (props = {}) => {
  const combinedProps = { ...defaultProps, ...props };
  
  return renderWithProviders(
    <FailedDocumentViewer {...combinedProps} />
  );
};

// Mock Blob and URL.createObjectURL
const mockBlob = vi.fn(() => ({
  text: () => Promise.resolve('mock text content'),
}));
global.Blob = mockBlob as any;

const mockCreateObjectURL = vi.fn(() => 'mock-object-url');
const mockRevokeObjectURL = vi.fn();
global.URL = {
  createObjectURL: mockCreateObjectURL,
  revokeObjectURL: mockRevokeObjectURL,
} as any;

describe('FailedDocumentViewer', () => {
  beforeEach(() => {
    setupTestEnvironment();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Loading State', () => {
    test('should show loading spinner initially', () => {
      // Mock API to never resolve
      vi.mocked(api.get).mockImplementation(() => new Promise(() => {}));
      
      renderFailedDocumentViewer();
      
      expect(screen.getByRole('progressbar')).toBeInTheDocument();
    });

    test('should show loading spinner with correct styling', () => {
      vi.mocked(api.get).mockImplementation(() => new Promise(() => {}));
      
      renderFailedDocumentViewer();
      
      const loadingContainer = screen.getByRole('progressbar').closest('div');
      expect(loadingContainer).toHaveStyle({
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        minHeight: '200px'
      });
    });
  });

  describe('Successful Document Loading', () => {
    test('should load and display PDF document', async () => {
      const mockResponse = {
        data: new Blob(['mock pdf content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/documents/failed/test-failed-doc-id/view', {
          responseType: 'blob'
        });
      });

      await waitFor(() => {
        const iframe = screen.getByTitle('test-document.pdf');
        expect(iframe).toBeInTheDocument();
        expect(iframe).toHaveAttribute('src', 'mock-object-url');
        expect(iframe).toHaveAttribute('width', '100%');
        expect(iframe).toHaveAttribute('height', '400px');
      });
    });

    test('should load and display image document', async () => {
      const mockResponse = {
        data: new Blob(['mock image content'], { type: 'image/jpeg' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        filename: 'test-image.jpg',
        mimeType: 'image/jpeg'
      });

      await waitFor(() => {
        const image = screen.getByAltText('test-image.jpg');
        expect(image).toBeInTheDocument();
        expect(image).toHaveAttribute('src', 'mock-object-url');
        expect(image).toHaveStyle({
          maxWidth: '100%',
          maxHeight: '400px',
          objectFit: 'contain',
        });
      });
    });

    test('should load and display text document', async () => {
      const mockResponse = {
        data: new Blob(['mock text content'], { type: 'text/plain' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        filename: 'test-file.txt',
        mimeType: 'text/plain'
      });

      await waitFor(() => {
        const iframe = screen.getByTitle('test-file.txt');
        expect(iframe).toBeInTheDocument();
        expect(iframe).toHaveAttribute('src', 'mock-object-url');
      });
    });

    test('should show unsupported file type message', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/unknown' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        filename: 'test-file.unknown',
        mimeType: 'application/unknown'
      });

      await waitFor(() => {
        expect(screen.getByText('Cannot preview this file type (application/unknown)')).toBeInTheDocument();
        expect(screen.getByText('File: test-file.unknown')).toBeInTheDocument();
        expect(screen.getByText('You can try downloading the file to view it locally.')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    test('should show 404 error when document not found', async () => {
      const error = {
        response: { status: 404 }
      };
      vi.mocked(api.get).mockRejectedValueOnce(error);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(screen.getByText('Document file not found or has been deleted')).toBeInTheDocument();
        expect(screen.getByText('The original file may have been deleted or moved from storage.')).toBeInTheDocument();
      });
    });

    test('should show generic error for other failures', async () => {
      const error = new Error('Network error');
      vi.mocked(api.get).mockRejectedValueOnce(error);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(screen.getByText('Failed to load document for viewing')).toBeInTheDocument();
        expect(screen.getByText('The original file may have been deleted or moved from storage.')).toBeInTheDocument();
      });
    });

    test('should handle API errors gracefully', async () => {
      const error = {
        response: { status: 500 }
      };
      vi.mocked(api.get).mockRejectedValueOnce(error);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(screen.getByText('Failed to load document for viewing')).toBeInTheDocument();
      });
    });
  });

  describe('Memory Management', () => {
    test('should create object URL when loading document', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(mockCreateObjectURL).toHaveBeenCalled();
      });

      // Should display the document
      await waitFor(() => {
        expect(screen.getByTitle(defaultProps.filename)).toBeInTheDocument();
      });
    });

    test('should create new object URL when failedDocumentId changes', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValue(mockResponse);

      const { rerender } = renderFailedDocumentViewer();

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/documents/failed/test-failed-doc-id/view', {
          responseType: 'blob'
        });
      });

      // Change the failedDocumentId
      rerender(
        <ThemeProvider theme={theme}>
          <FailedDocumentViewer
            failedDocumentId="new-doc-id"
            filename="test-document.pdf"
            mimeType="application/pdf"
          />
        </ThemeProvider>
      );

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/documents/failed/new-doc-id/view', {
          responseType: 'blob'
        });
      });

      expect(api.get).toHaveBeenCalledTimes(2);
    });
  });

  describe('Document Types', () => {
    test('should handle PDF documents correctly', async () => {
      const mockResponse = {
        data: new Blob(['mock pdf content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        mimeType: 'application/pdf'
      });

      await waitFor(() => {
        const iframe = screen.getByTitle(defaultProps.filename);
        expect(iframe).toBeInTheDocument();
        expect(iframe.tagName).toBe('IFRAME');
      });
    });

    test('should handle various image types', async () => {
      const imageTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];

      for (const mimeType of imageTypes) {
        const mockResponse = {
          data: new Blob(['mock image content'], { type: mimeType }),
        };
        vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

        const filename = `test.${mimeType.split('/')[1]}`;
        renderFailedDocumentViewer({
          filename,
          mimeType
        });

        await waitFor(() => {
          const image = screen.getByAltText(filename);
          expect(image).toBeInTheDocument();
          expect(image.tagName).toBe('IMG');
        });

        // Clean up for next iteration
        screen.getByAltText(filename).remove();
      }
    });

    test('should handle text documents', async () => {
      const textTypes = ['text/plain', 'text/html', 'text/css'];

      for (const mimeType of textTypes) {
        const mockResponse = {
          data: new Blob(['mock text content'], { type: mimeType }),
        };
        vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

        const filename = `test.${mimeType.split('/')[1]}`;
        renderFailedDocumentViewer({
          filename,
          mimeType
        });

        await waitFor(() => {
          const iframe = screen.getByTitle(filename);
          expect(iframe).toBeInTheDocument();
          expect(iframe.tagName).toBe('IFRAME');
        });

        // Clean up for next iteration
        screen.getByTitle(filename).remove();
      }
    });
  });

  describe('Styling and Layout', () => {
    test('should apply correct Paper styling', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        const paper = screen.getByTitle(defaultProps.filename).closest('.MuiPaper-root');
        expect(paper).toHaveClass('MuiPaper-root');
      });
    });

    test('should center images properly', async () => {
      const mockResponse = {
        data: new Blob(['mock image content'], { type: 'image/jpeg' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        mimeType: 'image/jpeg'
      });

      await waitFor(() => {
        const imageContainer = screen.getByAltText(defaultProps.filename).closest('div');
        expect(imageContainer).toHaveStyle({
          textAlign: 'center'
        });
      });
    });
  });

  describe('API Call Parameters', () => {
    test('should call API with correct endpoint and parameters', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/documents/failed/test-failed-doc-id/view', {
          responseType: 'blob'
        });
      });
    });

    test('should handle different document IDs correctly', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        failedDocumentId: 'different-doc-id'
      });

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/documents/failed/different-doc-id/view', {
          responseType: 'blob'
        });
      });
    });
  });

  describe('Edge Cases', () => {
    test('should handle empty blob response', async () => {
      const mockResponse = {
        data: new Blob([], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        // Should still create object URL and show iframe
        expect(mockCreateObjectURL).toHaveBeenCalled();
        expect(screen.getByTitle(defaultProps.filename)).toBeInTheDocument();
      });
    });

    test('should handle very long filenames', async () => {
      const longFilename = 'a'.repeat(500) + '.pdf';
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        filename: longFilename
      });

      await waitFor(() => {
        expect(screen.getByTitle(longFilename)).toBeInTheDocument();
      });
    });

    test('should handle special characters in filename', async () => {
      const specialFilename = 'test file & "quotes" <brackets>.pdf';
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        filename: specialFilename
      });

      await waitFor(() => {
        expect(screen.getByTitle(specialFilename)).toBeInTheDocument();
      });
    });

    test('should handle undefined or null mimeType gracefully', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: '' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        mimeType: undefined as any
      });

      await waitFor(() => {
        // Should show unsupported file type message
        expect(screen.getByText(/Cannot preview this file type \(unknown\)/)).toBeInTheDocument();
      });
    });
  });

  describe('Accessibility', () => {
    test('should have proper ARIA attributes', async () => {
      const mockResponse = {
        data: new Blob(['mock content'], { type: 'application/pdf' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer();

      await waitFor(() => {
        const iframe = screen.getByTitle(defaultProps.filename);
        expect(iframe).toHaveAttribute('title', defaultProps.filename);
      });
    });

    test('should have proper alt text for images', async () => {
      const mockResponse = {
        data: new Blob(['mock image content'], { type: 'image/jpeg' }),
      };
      vi.mocked(api.get).mockResolvedValueOnce(mockResponse);

      renderFailedDocumentViewer({
        mimeType: 'image/jpeg'
      });

      await waitFor(() => {
        const image = screen.getByAltText(defaultProps.filename);
        expect(image).toBeInTheDocument();
      });
    });
  });
});