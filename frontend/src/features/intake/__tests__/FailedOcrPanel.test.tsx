import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { FailedOcrPanel } from '../attention/FailedOcrPanel';
import { isLit, acknowledge } from '../../board/litStore';
import { documentService, ocrService, ok, serveDefaults } from './intakeMocks';
import { failedDoc, failedList, renderIntake, resetIntakeState } from './intakeTestUtils';

function serveFailed(docs: unknown[]) {
  ocrService.listFailedDocuments.mockImplementation(() => ok(failedList(docs)));
}

const board = () => screen.findByRole('grid', { name: 'Failed documents' });

async function openDoc(user: ReturnType<typeof userEvent.setup>, name: string) {
  const grid = await board();
  await user.click(within(grid).getByRole('rowheader', { name: new RegExp(name.replace('.', '\\.')) }));
  return screen.findByRole('dialog', { name });
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  serveFailed([failedDoc('scan1'), failedDoc('scan2', { failure_reason: 'low_ocr_confidence', failure_category: 'Low OCR Confidence', ocr_confidence: 42.46, ocr_word_count: 12 })]);
});

describe('Failed OCR board', () => {
  it('lists failed documents with a FAILED mark, reason, stage, retries and the error line', async () => {
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /scan1\.pdf/ });
    expect(row).toHaveTextContent(/failed/i);
    expect(row).toHaveTextContent('Timeout');
    expect(row).toHaveTextContent('OCR');
    expect(row).toHaveAccessibleDescription('Tesseract timed out');
  });

  it('marks new failures, with a tag, until the row is opened', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /scan1\.pdf/ });
    expect(row).toHaveAttribute('data-changed', 'true');
    expect(within(row).getByText('Changed')).toBeInTheDocument();
    expect(isLit('attention', 'document:scan1')).toBe(true);
    await openDoc(user, 'scan1.pdf');
    expect(isLit('attention', 'document:scan1')).toBe(false);
  });

  it('does not mark an acknowledged failure again on reload', async () => {
    const first = renderIntake(<FailedOcrPanel />);
    await board();
    acknowledge('attention', 'document:scan1');
    first.unmount();
    renderIntake(<FailedOcrPanel />);
    await board();
    expect(isLit('attention', 'document:scan1')).toBe(false);
    expect(isLit('attention', 'document:scan2')).toBe(true);
  });

  it('filters by stage and reason on the server and clears the filters', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    await board();
    await user.click(screen.getByRole('button', { name: /stage/i }));
    await user.click(screen.getByRole('option', { name: 'Ingestion' }));
    await waitFor(() => expect(ocrService.listFailedDocuments).toHaveBeenLastCalledWith(expect.objectContaining({ stage: 'ingestion', offset: 0 })));
    await user.click(screen.getByRole('button', { name: /all reasons/i }));
    await user.click(screen.getByRole('option', { name: 'OCR timed out' }));
    await waitFor(() => expect(ocrService.listFailedDocuments).toHaveBeenLastCalledWith(expect.objectContaining({ reason: 'ocr_timeout' })));
    await user.click(screen.getByRole('button', { name: 'Clear filters' }));
    await waitFor(() => expect(ocrService.listFailedDocuments).toHaveBeenLastCalledWith(expect.objectContaining({ stage: undefined, reason: undefined })));
  });

  it('shows the empty state', async () => {
    serveFailed([]);
    renderIntake(<FailedOcrPanel />);
    expect(await screen.findByRole('heading', { name: 'No failed documents' })).toBeInTheDocument();
  });

  it('shows a retryable error', async () => {
    ocrService.listFailedDocuments.mockImplementation(() => Promise.reject(new Error('down')));
    renderIntake(<FailedOcrPanel />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load failed documents.');
  });
});

describe('Failed OCR bulk actions', () => {
  it('retries the selected documents through the bulk retry dialog', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(within(grid).getByRole('row', { name: /scan1\.pdf/ })).getByRole('checkbox'));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Retry' }));
    const dialog = screen.getByRole('dialog', { name: 'Bulk OCR retry' });
    expect(within(dialog).getByRole('radio', { name: /selected documents \(1 selected\)/ })).toBeChecked();
    await user.click(within(dialog).getByRole('button', { name: 'Preview' }));
    await waitFor(() => expect(documentService.bulkRetryOcr).toHaveBeenCalledWith({ mode: 'specific', preview_only: true, document_ids: ['scan1'] }));
    await user.click(within(dialog).getByRole('button', { name: 'Retry 2 documents' }));
    await waitFor(() => expect(documentService.bulkRetryOcr).toHaveBeenLastCalledWith({ mode: 'specific', preview_only: false, document_ids: ['scan1'] }));
    expect(await screen.findByText('2 of 2 documents queued')).toBeInTheDocument();
  });

  it('retries the selection with chosen OCR languages', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(within(grid).getByRole('row', { name: /scan2\.pdf/ })).getByRole('checkbox'));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Retry' }));
    const dialog = screen.getByRole('dialog', { name: 'Bulk OCR retry' });
    await user.click(within(dialog).getByRole('button', { name: /add language|choose languages|languages/i }));
    await user.click(await within(dialog).findByRole('checkbox', { name: /German/ }));
    await user.click(within(dialog).getByRole('button', { name: 'Preview' }));
    await user.click(await within(dialog).findByRole('button', { name: 'Retry 2 documents' }));
    await waitFor(() => expect(ocrService.retryWithLanguage).toHaveBeenCalledWith('scan2', undefined, ['deu']));
  });

  it('deletes the selected documents only after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Delete 2 documents?' });
    expect(documentService.bulkDelete).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Delete documents' }));
    await waitFor(() => expect(documentService.bulkDelete).toHaveBeenCalledWith(['scan1', 'scan2']));
    expect(isLit('attention', 'document:scan1')).toBe(false);
  });
});

describe('Failure details panel (ported from DocumentManagementPage runtime checks)', () => {
  it('opens with the failure summary, file facts and dates', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    expect(within(panel).getByRole('group', { name: 'Failure' })).toHaveTextContent('Timeout');
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('application/pdf');
    expect(within(panel).getByRole('group', { name: 'Dates' })).toHaveTextContent('none yet');
  });

  it('shows the recorded error message', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    expect(within(panel).getByText('Tesseract timed out')).toBeInTheDocument();
  });

  it('handles a missing error message', async () => {
    serveFailed([failedDoc('scan1', { error_message: null })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    expect(within(panel).getByText('No error message was recorded.')).toBeInTheDocument();
  });

  it('falls back to the reason when the server sends no category', async () => {
    serveFailed([failedDoc('scan1', { failure_category: undefined, failure_reason: 'pdf_parsing_error' })]);
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    expect(within(grid).getByRole('row', { name: /scan1\.pdf/ })).toHaveTextContent('PDF could not be read');
  });

  it('shows OCR confidence and word count for low-confidence failures', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan2.pdf');
    const result = within(panel).getByRole('group', { name: 'OCR result' });
    expect(result).toHaveTextContent('42.5%');
    expect(result).toHaveTextContent('12');
  });

  it.each([null, undefined])('handles a %s OCR confidence without crashing', async (value) => {
    serveFailed([failedDoc('scan2', { failure_reason: 'low_ocr_confidence', ocr_confidence: value, ocr_word_count: value })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan2.pdf');
    expect(within(panel).getByRole('group', { name: 'OCR result' })).toHaveTextContent('—');
  });

  it('does not nest blocks inside paragraphs', async () => {
    serveFailed([failedDoc('scan2', { failure_reason: 'low_ocr_confidence', ocr_confidence: 25.5, tags: ['tag1', 'tag2'] })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan2.pdf');
    expect(panel.querySelectorAll('p div, p p, p ul, p pre')).toHaveLength(0);
    expect(within(panel).getByText('Tags: tag1, tag2')).toBeInTheDocument();
  });

  it('copes with edge-case sizes and zero values', async () => {
    serveFailed([failedDoc('zero', { file_size: 0, ocr_confidence: 0, ocr_word_count: 0, failure_reason: 'low_ocr_confidence', error_message: '' })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'zero.pdf');
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('0 B');
    expect(within(panel).getByRole('group', { name: 'OCR result' })).toHaveTextContent('0.0%');
  });

  it('copes with missing timestamps and sizes', async () => {
    serveFailed([failedDoc('bare', { file_size: null, last_retry_at: null, updated_at: 'not a date', mime_type: null })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'bare.pdf');
    expect(within(panel).getByRole('group', { name: 'Dates' })).toHaveTextContent('—');
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('0 B');
  });

  it('retries OCR from the panel and clears the mark', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    await user.click(within(panel).getByRole('button', { name: 'Retry OCR' }));
    await waitFor(() => expect(documentService.retryOcr).toHaveBeenCalledWith('scan1'));
    expect(await screen.findByText('OCR retry queued')).toBeInTheDocument();
  });

  it('shows the retry history and the file preview state', async () => {
    documentService.getDocumentRetryHistory.mockImplementation(() =>
      ok({
        document_id: 'scan1',
        total_retries: 1,
        retry_history: [{ id: 'r1', retry_reason: 'manual_retry', priority: 12, queue_id: 'abcdef123456', created_at: '2026-04-01T10:00:00Z', previous_status: 'failed' }],
      }),
    );
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    const history = await within(panel).findByRole('grid', { name: 'OCR retry history' });
    const row = within(history).getAllByRole('row').find((r) => /Manual retry/.test(r.textContent ?? ''));
    expect(row).toHaveTextContent('High (12)');
    expect(row).toHaveTextContent('Queued · abcdef12');
    expect(await within(panel).findByText('The file was not found or has been deleted.')).toBeInTheDocument();
  });
});
