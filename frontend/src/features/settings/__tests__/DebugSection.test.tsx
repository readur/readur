import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import DebugSection from '../debug/DebugSection';
import { documentService } from '../../../services/api';
import { apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const DEBUG_INFO = {
  document_id: 'doc-1',
  filename: 'invoice.pdf',
  overall_status: 'failed',
  debug_timestamp: '2026-09-01T10:00:00Z',
  user_settings: { enable_background_ocr: true, ocr_min_confidence: 30, max_file_size_mb: 50 },
  pipeline_steps: [
    {
      step: 1,
      name: 'File Upload & Ingestion',
      status: 'completed',
      success: true,
      details: {
        filename: 'invoice.pdf',
        original_filename: 'Invoice.pdf',
        file_size: 2097152,
        mime_type: 'application/pdf',
        file_exists: true,
        created_at: '2026-09-01T09:00:00Z',
        file_analysis: {
          file_type: 'pdf',
          file_size_bytes: 2097152,
          is_readable: true,
          pdf_info: { is_valid_pdf: true, page_count: 3, has_text_content: false, has_images: true, is_encrypted: false, font_count: 2, estimated_text_length: 0 },
        },
      },
    },
    { step: 2, name: 'OCR Queue Enrollment', status: 'completed', success: true, details: { user_ocr_enabled: true, queue_entries_count: 1, queue_history: [{ status: 'failed', priority: 5, created_at: '2026-09-01T09:00:00Z', attempts: 2, worker_id: 'w1' }] } },
    { step: 3, name: 'OCR Processing', status: 'failed', success: false, error: 'Tesseract crashed', details: { ocr_text_length: 0, has_processed_image: false } },
    {
      step: 4,
      name: 'Quality Validation',
      status: 'not_reached',
      success: false,
      details: {
        quality_thresholds: { min_confidence: 30, brightness_threshold: 40, contrast_threshold: 0.15, noise_threshold: 0.3, sharpness_threshold: 0.15 },
        actual_values: { confidence: null, word_count: 0, processed_image_available: false },
        quality_checks: { confidence_check: false, word_count_check: null },
      },
    },
  ],
  failed_document_info: { failure_reason: 'ocr_error', failure_stage: 'ocr', retry_count: 1, created_at: '2026-09-01T09:05:00Z', error_message: 'Tesseract crashed' },
  detailed_processing_logs: [{ id: 'l1', status: 'failed', priority: 5, created_at: '2026-09-01T09:00:00Z', attempts: 2, error_message: 'boom' }],
};

const render = () => renderSettings(<DebugSection />, { path: '/settings/debug' });

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(DEBUG_INFO));
});

afterEach(() => {
  vi.useRealTimers();
});

describe('DebugSection', () => {
  it('renders the upload and search tabs without results', () => {
    render();
    expect(screen.getByRole('tab', { name: 'Upload & Debug' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: 'Search Existing' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Debug Results' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Upload Document for Debug Analysis' })).toBeInTheDocument();
  });

  it('looks up a document and opens the results with the pipeline stepper', async () => {
    const user = userEvent.setup();
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    const button = screen.getByRole('button', { name: 'Debug' });
    expect(button).toBeDisabled();
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'doc-1');
    await user.click(button);
    expect(apiMock.get).toHaveBeenCalledWith('/documents/doc-1/debug');
    expect(await screen.findByRole('tab', { name: 'Debug Results' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('heading', { name: 'Document: invoice.pdf' })).toBeInTheDocument();
    const stepper = screen.getByRole('heading', { name: 'Processing Pipeline' }).closest('section') as HTMLElement;
    const items = Array.from(within(stepper).getAllByRole('list')[0].children) as HTMLElement[];
    expect(items).toHaveLength(4);
    expect(items[2]).toHaveTextContent('OCR Processing');
    expect(items[2]).toHaveTextContent('failed');
    expect(within(items[2]).getByText('Tesseract crashed')).toBeInTheDocument();
  });

  it('shows the diagnostics below the pipeline', async () => {
    const user = userEvent.setup();
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'doc-1{Enter}');
    expect(await screen.findByRole('heading', { name: 'Failed Document Information' })).toBeInTheDocument();
    expect(screen.getByText('Detailed Processing Logs')).toBeInTheDocument();
    expect(screen.getByText('boom')).toBeInTheDocument();
    expect(screen.getByText('Detailed File Analysis')).toBeInTheDocument();
    expect(screen.getByText('Queue History')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'User Settings' }));
    expect(screen.getByText('OCR Settings')).toBeInTheDocument();
  });

  it('loads the original and processed images through the signed-in client', async () => {
    const withImage = {
      ...DEBUG_INFO,
      pipeline_steps: DEBUG_INFO.pipeline_steps.map((step) =>
        step.step === 3 ? { ...step, details: { ...step.details, has_processed_image: true } } : step,
      ),
    };
    apiMock.get.mockResolvedValue(ok(withImage));
    const view = vi.spyOn(documentService, 'view').mockResolvedValue({ data: new Blob(['%PDF']) } as never);
    const processed = vi
      .spyOn(documentService, 'getProcessedImage')
      .mockRejectedValue(httpError(404));
    const user = userEvent.setup();
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'doc-1{Enter}');
    expect(await screen.findByRole('heading', { name: 'Processed Images' })).toBeInTheDocument();
    await waitFor(() => expect(processed).toHaveBeenCalledWith('doc-1'));
    expect(view).toHaveBeenCalledWith('doc-1');
    expect(await screen.findByText('Processed image not available')).toBeInTheDocument();
    expect(document.querySelector('[src^="/api/"]')).toBeNull();
  });

  it('handles missing optional data without crashing', async () => {
    apiMock.get.mockResolvedValue(
      ok({ document_id: 'd', filename: 'x.txt', overall_status: 'completed', debug_timestamp: '2026-09-01T00:00:00Z', user_settings: null, pipeline_steps: undefined }),
    );
    const user = userEvent.setup();
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'd{Enter}');
    expect(await screen.findByRole('heading', { name: 'Document: x.txt' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Failed Document Information' })).not.toBeInTheDocument();
  });

  it('reports a server error', async () => {
    apiMock.get.mockRejectedValue(httpError(500, { message: 'debug exploded' }));
    const user = userEvent.setup();
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'doc-1{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent('debug exploded');
  });

  it('retries a missing document before reporting it', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    apiMock.get.mockRejectedValue(httpError(404));
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render();
    await user.click(screen.getByRole('tab', { name: 'Search Existing' }));
    await user.type(screen.getByRole('textbox', { name: 'Document ID' }), 'gone{Enter}');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(7000);
    });
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledTimes(4));
    expect(await screen.findByRole('alert')).toHaveTextContent('Document gone not found');
  });

  it('uploads a file and follows OCR until it completes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    apiMock.post.mockResolvedValue(ok({ id: 'new-doc' }));
    apiMock.get.mockImplementation(async (url: string) =>
      url === '/documents/new-doc' ? ok({ id: 'new-doc', ocr_status: 'completed' }) : ok(DEBUG_INFO),
    );
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const { container } = render();
    const file = new File(['%PDF'], 'scan.pdf', { type: 'application/pdf' });
    await user.upload(container.querySelector('input[type="file"]') as HTMLInputElement, file);
    expect(screen.getByText('scan.pdf')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Upload & Debug' }));
    await waitFor(() => expect(apiMock.post).toHaveBeenCalledWith('/documents', expect.any(FormData), expect.any(Object)));
    expect(await screen.findByText('new-doc')).toBeInTheDocument();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(4500);
    });
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledWith('/documents/new-doc/debug'));
    expect(await screen.findByRole('tab', { name: 'Debug Results' })).toHaveAttribute('aria-selected', 'true');
    // View Document opens the document drawer (a bare /api URL would carry no sign-in and fail).
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    await user.click(screen.getByRole('tab', { name: 'Upload & Debug' }));
    await user.click(await screen.findByRole('button', { name: 'View Document' }));
    expect(open).not.toHaveBeenCalled();
    expect(screen.getByRole('status', { name: 'location', hidden: true })).toHaveTextContent('/settings/debug?document=new-doc');
    open.mockRestore();
  });
});
