import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { AttentionSection } from '../attention/AttentionSection';
import { DEFAULT_THRESHOLD } from '../attention/LowConfidencePanel';
import { documentService, ok, queueService, serveDefaults } from './intakeMocks';
import { renderIntake, resetIntakeState } from './intakeTestUtils';

const segments = () => screen.getByRole('radiogroup', { name: 'Show' });
const location = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;

async function show(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(within(segments()).getByRole('radio', { name }));
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
});

describe('Needs attention: segmented filter (ported from DocumentManagementPage)', () => {
  it('renders without crashing and shows the failed documents first', async () => {
    renderIntake(<AttentionSection />, { path: '/intake?section=attention' });
    expect(within(segments()).getByRole('radio', { name: 'Failed OCR' })).toBeChecked();
    expect(await screen.findByRole('grid', { name: 'Failed documents' })).toBeInTheDocument();
  });

  it('has the four views in order', () => {
    renderIntake(<AttentionSection />);
    expect(within(segments()).getAllByRole('radio').map((r) => r.textContent)).toEqual([
      'Failed OCR',
      'Low confidence',
      'Duplicates',
      'Cleanup',
    ]);
  });

  it('writes the view to the URL and restores it', async () => {
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?section=attention' });
    await show(user, 'Duplicates');
    expect(location()).toBe('/intake?section=attention&view=duplicates');
    expect(await screen.findByRole('grid', { name: 'Duplicate documents' })).toBeInTheDocument();
  });

  it('opens the view named in the URL', () => {
    renderIntake(<AttentionSection />, { path: '/intake?section=attention&view=cleanup' });
    expect(within(segments()).getByRole('radio', { name: 'Cleanup' })).toBeChecked();
  });

  it('survives rapid switching between views', async () => {
    const user = userEvent.setup();
    renderIntake(<AttentionSection />);
    for (const name of ['Low confidence', 'Duplicates', 'Cleanup', 'Failed OCR', 'Cleanup']) {
      await show(user, name);
    }
    expect(within(segments()).getByRole('radio', { name: 'Cleanup' })).toBeChecked();
    expect(screen.getByRole('heading', { name: 'Retry OCR for all documents' })).toBeInTheDocument();
  });

  it('offers a refresh on the failed view', async () => {
    renderIntake(<AttentionSection />);
    expect(await screen.findByRole('button', { name: 'Refresh' })).toBeInTheDocument();
  });
});

describe('Needs attention: low confidence', () => {
  it('shows the threshold slider at 30% by default', () => {
    renderIntake(<AttentionSection />, { path: '/intake?view=lowConfidence' });
    const slider = screen.getByRole('slider', { name: 'Confidence threshold' });
    expect(slider).toHaveValue(String(DEFAULT_THRESHOLD));
    expect(screen.getByText('30%')).toBeInTheDocument();
  });

  it('explains that matches are previewed before deleting', () => {
    renderIntake(<AttentionSection />, { path: '/intake?view=lowConfidence' });
    expect(screen.getByText(/Preview the matches before deleting anything/)).toBeInTheDocument();
  });

  it('keeps Delete disabled until a preview found matches', () => {
    renderIntake(<AttentionSection />, { path: '/intake?view=lowConfidence' });
    expect(screen.getByRole('button', { name: 'Preview matches' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Delete matches' })).toBeDisabled();
  });

  it('previews with the chosen threshold, then deletes after confirmation', async () => {
    documentService.deleteLowConfidence.mockImplementation((_t: number, preview: boolean) =>
      ok(
        preview
          ? {
              message: '2 documents below 45%',
              matched_count: 2,
              documents: [
                { id: 'd1', filename: 'a.pdf', file_size: 1024, ocr_confidence: 12.34, ocr_status: 'completed', created_at: '2026-01-01T00:00:00Z' },
                { id: 'd2', filename: 'b.pdf', file_size: 0, ocr_confidence: null, ocr_status: 'failed', created_at: '2026-01-01T00:00:00Z' },
              ],
            }
          : { message: 'Deleted 2 documents', matched_count: 2, deleted_count: 2 },
      ),
    );
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=lowConfidence' });
    const slider = screen.getByRole('slider', { name: 'Confidence threshold' });
    slider.focus();
    for (let i = 0; i < 15; i += 1) await user.keyboard('{ArrowRight}');
    await user.click(screen.getByRole('button', { name: 'Preview matches' }));
    await waitFor(() => expect(documentService.deleteLowConfidence).toHaveBeenCalledWith(45, true));
    const results = await screen.findByRole('grid', { name: 'Preview results' });
    expect(within(results).getByRole('row', { name: /a\.pdf/ })).toHaveTextContent('12.3%');
    expect(within(results).getByRole('row', { name: /b\.pdf/ })).toHaveTextContent('n/a');
    await user.click(screen.getByRole('button', { name: 'Delete matches' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Delete 2 documents below 45%?' });
    await user.click(within(confirm).getByRole('button', { name: 'Delete documents' }));
    await waitFor(() => expect(documentService.deleteLowConfidence).toHaveBeenCalledWith(45, false));
    expect(await screen.findByText('Deleted 2 documents')).toBeInTheDocument();
  });

  it('asks for a new preview when the threshold changes after previewing', async () => {
    documentService.deleteLowConfidence.mockImplementation(() => ok({ message: '1 document', matched_count: 1, documents: [] }));
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=lowConfidence' });
    await user.click(screen.getByRole('button', { name: 'Preview matches' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Delete matches' })).toBeEnabled());
    screen.getByRole('slider').focus();
    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('button', { name: 'Delete matches' })).toBeDisabled();
    expect(screen.getByText('The threshold changed. Preview again before deleting.')).toBeInTheDocument();
  });
});

describe('Needs attention: duplicates', () => {
  it('lists duplicates grouped by content hash', async () => {
    documentService.getDuplicates.mockImplementation(() =>
      ok({
        duplicates: [
          {
            file_hash: 'abcdef0123456789abcdef',
            duplicate_count: 2,
            first_uploaded: '2026-01-01T00:00:00Z',
            last_uploaded: '2026-01-02T00:00:00Z',
            documents: [
              { id: 'd1', filename: 'invoice.pdf', original_filename: 'invoice.pdf', file_size: 1024, mime_type: 'application/pdf', created_at: '2026-01-01T00:00:00Z' },
              { id: 'd2', filename: 'invoice (1).pdf', original_filename: 'invoice-copy.pdf', file_size: 1024, mime_type: 'application/pdf', created_at: '2026-01-02T00:00:00Z' },
            ],
          },
        ],
        pagination: { total: 1, limit: 25, offset: 0, has_more: false },
        statistics: { total_duplicate_groups: 1 },
      }),
    );
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=duplicates' });
    const grid = await screen.findByRole('grid', { name: 'Duplicate documents' });
    expect(within(grid).getByRole('row', { name: /invoice\.pdf/ })).toHaveTextContent('abcdef012345… 1/2');
    const copy = within(grid).getByRole('row', { name: /invoice \(1\)\.pdf/ });
    expect(copy).toHaveTextContent('abcdef012345… 2/2');
    expect(copy).toHaveAccessibleDescription('Original name: invoice-copy.pdf');
    expect(screen.getByText(/^1 groups? of identical files$/)).toBeInTheDocument();
    await user.click(within(copy).getByRole('button', { name: 'Download invoice (1).pdf' }));
    expect(documentService.downloadFile).toHaveBeenCalledWith('d2', 'invoice-copy.pdf');
  });

  it('shows the empty state', async () => {
    renderIntake(<AttentionSection />, { path: '/intake?view=duplicates' });
    expect(await screen.findByRole('heading', { name: 'No duplicates' })).toBeInTheDocument();
  });
});

describe('Needs attention: cleanup (each action behind a confirmation)', () => {
  it('previews failed documents, then deletes them after confirmation', async () => {
    documentService.deleteFailedOcr.mockImplementation((preview: boolean) =>
      ok(preview ? { message: '3 failed documents', matched_count: 3, document_ids: ['x1', 'x2', 'x3'] } : { message: 'Deleted 3', matched_count: 3 }),
    );
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=cleanup' });
    expect(screen.getByRole('button', { name: 'Delete failed documents' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Preview' }));
    expect(await screen.findByText('x1, x2, x3')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Delete failed documents' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Delete 3 documents whose OCR failed?' });
    expect(documentService.deleteFailedOcr).toHaveBeenCalledTimes(1);
    await user.click(within(confirm).getByRole('button', { name: 'Delete failed documents' }));
    await waitFor(() => expect(documentService.deleteFailedOcr).toHaveBeenLastCalledWith(false));
  });

  it('retries every document only after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=cleanup' });
    await user.click(screen.getByRole('button', { name: 'Retry all documents' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Retry OCR for every document?' });
    expect(documentService.bulkRetryOcr).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Retry all documents' }));
    await waitFor(() => expect(documentService.bulkRetryOcr).toHaveBeenCalledWith({ mode: 'all', preview_only: false }));
  });

  it('re-queues failed jobs only after confirmation, and can be cancelled', async () => {
    const user = userEvent.setup();
    renderIntake(<AttentionSection />, { path: '/intake?view=cleanup' });
    await user.click(screen.getByRole('button', { name: 'Retry failed jobs' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Retry failed jobs?' })).getByRole('button', { name: 'Cancel' }));
    expect(queueService.requeueFailed).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Retry failed jobs' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Retry failed jobs?' })).getByRole('button', { name: 'Retry jobs' }));
    await waitFor(() => expect(queueService.requeueFailed).toHaveBeenCalled());
  });
});
