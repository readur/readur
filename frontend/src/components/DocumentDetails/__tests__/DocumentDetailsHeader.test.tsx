import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import DocumentDetailsHeader from '../DocumentDetailsHeader';
import type { Document } from '../../../services/api';

const theme = createTheme();

/**
 * Creates a mock translation function that returns human-readable labels
 * based on the i18n key's last segment.
 */
const createMockT = () =>
  vi.fn((key: string, options?: any) => {
    const translations: Record<string, string> = {
      'documents.ocrStatus.pending': 'Queued',
      'documents.ocrStatus.processing': 'Processing...',
      'documents.ocrStatus.done': 'OCR Done',
      'documents.ocrStatus.failed': 'OCR Failed',
      'documentDetails.actions.backToDocuments': 'Back to Documents',
      'documentDetails.actions.download': 'Download',
      'documentDetails.actions.deleteDocument': 'Delete Document',
    };
    return translations[key] || key;
  });

/**
 * Creates a base mock Document with sensible defaults.
 */
const createMockDocument = (overrides: Partial<Document> = {}): Document => ({
  id: 'doc-1',
  filename: 'report.pdf',
  original_filename: 'report.pdf',
  file_path: '/uploads/report.pdf',
  file_size: 2048000,
  mime_type: 'application/pdf',
  tags: [],
  created_at: '2025-06-15T10:00:00Z',
  updated_at: '2025-06-15T10:00:00Z',
  user_id: 'user-1',
  username: 'testuser',
  has_ocr_text: false,
  ...overrides,
});

/**
 * Default props shared by all renders. Individual tests override specific fields.
 */
const createDefaultProps = (overrides: Partial<Parameters<typeof DocumentDetailsHeader>[0]> = {}) => ({
  document: createMockDocument(),
  documentLabels: [],
  deleting: false,
  retryingOcr: false,
  onBack: vi.fn(),
  onDownload: vi.fn(),
  onDelete: vi.fn(),
  onRetryOcr: vi.fn(),
  onEditLabels: vi.fn(),
  formatFileSize: (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`,
  formatDate: (d: string) => new Date(d).toLocaleDateString(),
  t: createMockT(),
  ...overrides,
});

/**
 * Renders DocumentDetailsHeader wrapped in MUI ThemeProvider.
 */
const renderHeader = (propOverrides: Partial<Parameters<typeof DocumentDetailsHeader>[0]> = {}) => {
  const props = createDefaultProps(propOverrides);
  return render(
    <ThemeProvider theme={theme}>
      <DocumentDetailsHeader {...(props as any)} />
    </ThemeProvider>
  );
};

describe('DocumentDetailsHeader - OCR status chip display', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders "Queued" chip when ocr_status is "pending"', () => {
    renderHeader({
      document: createMockDocument({ ocr_status: 'pending' }),
    });

    expect(screen.getByText('Queued')).toBeInTheDocument();
  });

  test('renders "Queued" chip when ocr_status is undefined', () => {
    renderHeader({
      document: createMockDocument({ ocr_status: undefined }),
    });

    expect(screen.getByText('Queued')).toBeInTheDocument();
  });

  test('renders "Processing..." chip without progress when ocr_status is "processing" and no progress data', () => {
    renderHeader({
      document: createMockDocument({ ocr_status: 'processing' }),
    });

    expect(screen.getByText('Processing...')).toBeInTheDocument();
    // No LinearProgress (determinate) should be rendered
    expect(screen.queryByRole('progressbar', { queryFallbackRole: true })).toBeFalsy;
  });

  test('renders "Processing... (3/10)" with progress bar when processing with progress data', () => {
    renderHeader({
      document: createMockDocument({
        ocr_status: 'processing',
        ocr_progress_current: 3,
        ocr_progress_total: 10,
      }),
    });

    expect(screen.getByText('Processing... (3/10)')).toBeInTheDocument();
    // A LinearProgress bar with determinate value should be rendered (value = 30%)
    const progressBars = screen.getAllByRole('progressbar');
    const linearProgress = progressBars.find(
      (el) => el.getAttribute('aria-valuenow') === '30'
    );
    expect(linearProgress).toBeDefined();
  });

  test('renders "OCR Done" chip when ocr_status is "completed"', () => {
    renderHeader({
      document: createMockDocument({ ocr_status: 'completed' }),
    });

    expect(screen.getByText('OCR Done')).toBeInTheDocument();
  });

  test('renders "OCR Failed" chip with retry button when ocr_status is "failed"', () => {
    const onRetryOcr = vi.fn();
    renderHeader({
      document: createMockDocument({ ocr_status: 'failed' }),
      onRetryOcr,
    });

    expect(screen.getByText('OCR Failed')).toBeInTheDocument();

    const retryButton = screen.getByRole('button', { name: /retry/i });
    expect(retryButton).toBeInTheDocument();
    expect(retryButton).not.toBeDisabled();
  });

  test('disables retry button when retryingOcr is true', () => {
    renderHeader({
      document: createMockDocument({ ocr_status: 'failed' }),
      retryingOcr: true,
    });

    const retryButton = screen.getByRole('button', { name: /retry/i });
    expect(retryButton).toBeDisabled();
  });
});
