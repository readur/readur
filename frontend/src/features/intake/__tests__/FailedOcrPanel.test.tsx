import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { FailedOcrPanel } from '../attention/FailedOcrPanel';
import { isLit, acknowledge } from '../../board/litStore';
import { documentService, ocrService, ok, serveDefaults } from './intakeMocks';
import { failedDoc, failedList, ocrDoc, ocrList, renderIntake, resetIntakeState } from './intakeTestUtils';

/** Real document ids differ from the failed-import record ids on purpose. */
const DOC1 = ocrDoc('doc-1', { filename: 'scan1.pdf', original_filename: 'scan1.pdf' });
const DOC2 = ocrDoc('doc-2', {
  filename: 'scan2.pdf',
  original_filename: 'scan2.pdf',
  ocr_failure_reason: 'low_ocr_confidence',
  failure_category: 'Low OCR Confidence',
  ocr_error: null,
  tags: ['tag1', 'tag2'],
  last_attempt_at: '2026-01-03T00:00:00Z',
});
const KEY1 = 'document:doc-1@2026-01-02T00:00:00Z';

function serveOcr(docs: unknown[]) {
  documentService.getFailedOcrDocuments.mockImplementation(() => ok(ocrList(docs)));
}
/** Behaves like GET /documents/failed: exact stage/reason filters, then limit/offset, total of the filtered set. */
function serveImports(records: Array<Record<string, unknown>>) {
  ocrService.listFailedDocuments.mockImplementation(
    (q: { stage?: string; reason?: string; limit?: number; offset?: number } = {}) => {
      const hits = records.filter(
        (r) => (!q.stage || r.failure_stage === q.stage) && (!q.reason || r.failure_reason === q.reason),
      );
      const offset = q.offset ?? 0;
      const page = hits.slice(offset, offset + (q.limit ?? 25));
      return ok({ ...failedList(page), pagination: { total: hits.length, limit: q.limit ?? 25, offset, total_pages: 1 } });
    },
  );
}

const board = () => screen.findByRole('grid', { name: 'Failed documents' });
const importsBoard = () => screen.findByRole('grid', { name: 'Other import failures' });

async function openDoc(user: ReturnType<typeof userEvent.setup>, name: string) {
  const grid = await board();
  await user.click(within(grid).getByRole('rowheader', { name: new RegExp(name.replace('.', '\\.')) }));
  return screen.findByRole('dialog', { name });
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  serveOcr([DOC1, DOC2]);
  serveImports([]);
});

describe('Failed OCR board', () => {
  it('lists documents from the failed-OCR endpoint with a FAILED mark, reason, retries and the error line', async () => {
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    expect(documentService.getFailedOcrDocuments).toHaveBeenCalledWith(25, 0);
    const row = within(grid).getByRole('row', { name: /scan1\.pdf/ });
    expect(row).toHaveTextContent(/failed/i);
    expect(row).toHaveTextContent('Timeout');
    expect(row).toHaveAccessibleDescription('Tesseract timed out');
  });

  it('marks new failures, with a tag, until the row is opened', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /scan1\.pdf/ });
    expect(row).toHaveAttribute('data-changed', 'true');
    expect(within(row).getByText('Changed')).toBeInTheDocument();
    expect(isLit('attention', KEY1)).toBe(true);
    await openDoc(user, 'scan1.pdf');
    expect(isLit('attention', KEY1)).toBe(false);
  });

  it('does not mark an acknowledged failure again on reload, but marks a newer failure', async () => {
    const first = renderIntake(<FailedOcrPanel />);
    await board();
    acknowledge('attention', KEY1);
    first.unmount();
    const second = renderIntake(<FailedOcrPanel />);
    await board();
    expect(isLit('attention', KEY1)).toBe(false);
    second.unmount();
    serveOcr([{ ...DOC1, updated_at: '2026-02-01T00:00:00Z' }]);
    renderIntake(<FailedOcrPanel />);
    await board();
    expect(isLit('attention', 'document:doc-1@2026-02-01T00:00:00Z')).toBe(true);
  });

  it('shows the empty state', async () => {
    serveOcr([]);
    renderIntake(<FailedOcrPanel />);
    expect(await screen.findByRole('heading', { name: 'No failed documents' })).toBeInTheDocument();
  });

  it('shows a retryable error', async () => {
    documentService.getFailedOcrDocuments.mockImplementation(() => Promise.reject(new Error('down')));
    renderIntake(<FailedOcrPanel />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load failed documents.');
  });
});

describe('Failed OCR bulk actions use real document ids', () => {
  it('retries the selected documents through the bulk retry dialog', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(within(grid).getByRole('row', { name: /scan1\.pdf/ })).getByRole('checkbox'));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Retry' }));
    const dialog = screen.getByRole('dialog', { name: 'Bulk OCR retry' });
    expect(within(dialog).getByRole('radio', { name: /selected documents \(1 selected\)/ })).toBeChecked();
    await user.click(within(dialog).getByRole('button', { name: 'Preview' }));
    await waitFor(() => expect(documentService.bulkRetryOcr).toHaveBeenCalledWith({ mode: 'specific', preview_only: true, document_ids: ['doc-1'] }));
    await user.click(within(dialog).getByRole('button', { name: 'Retry 2 documents' }));
    await waitFor(() => expect(documentService.bulkRetryOcr).toHaveBeenLastCalledWith({ mode: 'specific', preview_only: false, document_ids: ['doc-1'] }));
    expect(await screen.findByText('2 of 2 documents queued')).toBeInTheDocument();
  });

  it('retries the selection with chosen OCR languages, per real document id', async () => {
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
    await waitFor(() => expect(ocrService.retryWithLanguage).toHaveBeenCalledWith('doc-2', undefined, ['deu']));
  });

  it('deletes the selected documents by document id, only after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Delete 2 documents?' });
    expect(documentService.bulkDelete).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Delete documents' }));
    await waitFor(() => expect(documentService.bulkDelete).toHaveBeenCalledWith(['doc-1', 'doc-2']));
    expect(await screen.findByText('2 of 2 documents deleted')).toBeInTheDocument();
    expect(isLit('attention', KEY1)).toBe(false);
  });

  it('reports an error when the server deleted nothing', async () => {
    documentService.bulkDelete.mockImplementation(() => ok({ deleted_count: 0, failed_count: 2, deleted_documents: [], failed_documents: ['doc-1', 'doc-2'] }));
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Delete 2 documents?' })).getByRole('button', { name: 'Delete documents' }));
    expect(await screen.findByText('No documents were deleted')).toBeInTheDocument();
    expect(screen.getByRole('alertdialog', { name: 'Delete 2 documents?' })).toBeInTheDocument();
    expect(isLit('attention', KEY1)).toBe(true);
  });

  it('reports a partial delete as a problem', async () => {
    documentService.bulkDelete.mockImplementation(() => ok({ deleted_count: 1, failed_count: 1, deleted_documents: ['doc-1'], failed_documents: ['doc-2'] }));
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Delete 2 documents?' })).getByRole('button', { name: 'Delete documents' }));
    const toast = await screen.findByText('1 of 2 documents deleted');
    expect(toast.closest('[role="alert"]')).not.toBeNull();
  });

  it('after a partial delete, clears only the deleted document and keeps the other marked and selected', async () => {
    documentService.bulkDelete.mockImplementation(() => ok({ deleted_count: 1, failed_count: 1, deleted_documents: ['doc-1'], failed_documents: ['doc-2'] }));
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    const key2 = 'document:doc-2@2026-01-03T00:00:00Z';
    expect(isLit('attention', key2)).toBe(true);
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Delete 2 documents?' })).getByRole('button', { name: 'Delete documents' }));
    await screen.findByText('1 of 2 documents deleted');
    expect(isLit('attention', KEY1)).toBe(false);
    expect(isLit('attention', key2)).toBe(true);
    const row2 = within(await board()).getByRole('row', { name: /scan2\.pdf/ });
    expect(within(row2).getByRole('checkbox')).toBeChecked();
    expect(within(within(await board()).getByRole('row', { name: /scan1\.pdf/ })).getByRole('checkbox')).not.toBeChecked();
    expect(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Delete' })).toBeInTheDocument();
  });

  it('keeps the retry dialog open with an error when nothing was queued', async () => {
    documentService.bulkRetryOcr.mockImplementation((req: { preview_only?: boolean }) =>
      ok({ success: true, message: '', queued_count: req.preview_only ? 2 : 0, matched_count: 2, documents: [], estimated_total_time_minutes: 1 }),
    );
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Retry' }));
    const dialog = screen.getByRole('dialog', { name: 'Bulk OCR retry' });
    await user.click(within(dialog).getByRole('button', { name: 'Preview' }));
    await user.click(await within(dialog).findByRole('button', { name: 'Retry 2 documents' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('No documents were queued');
    expect(screen.getByRole('dialog', { name: 'Bulk OCR retry' })).toBeInTheDocument();
  });

  it('reports a partially queued retry as a problem', async () => {
    documentService.bulkRetryOcr.mockImplementation(() =>
      ok({ success: true, message: '', queued_count: 1, matched_count: 2, documents: [], estimated_total_time_minutes: 1 }),
    );
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await board();
    await user.click(within(grid).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Retry' }));
    const dialog = screen.getByRole('dialog', { name: 'Bulk OCR retry' });
    await user.click(within(dialog).getByRole('button', { name: 'Preview' }));
    await user.click(await within(dialog).findByRole('button', { name: 'Retry 2 documents' }));
    expect(await screen.findByText('Some documents were not queued. Check them and try again.')).toBeInTheDocument();
  });
});

describe('Failure details panel (document actions)', () => {
  it('opens with the failure summary, file facts and dates', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    expect(within(panel).getByRole('group', { name: 'Failure' })).toHaveTextContent('Timeout');
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('application/pdf');
    expect(within(panel).getByRole('group', { name: 'Dates' })).toHaveTextContent('none yet');
  });

  it('shows the recorded OCR error', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    expect(within(panel).getByText('Tesseract timed out')).toBeInTheDocument();
  });

  it('handles a missing error message and shows tags', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan2.pdf');
    expect(within(panel).getByText('No error message was recorded.')).toBeInTheDocument();
    expect(within(panel).getByText('Tags: tag1, tag2')).toBeInTheDocument();
    expect(panel.querySelectorAll('p div, p p, p ul, p pre')).toHaveLength(0);
  });

  it('retries OCR with the document id', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    await user.click(within(panel).getByRole('button', { name: 'Retry OCR' }));
    await waitFor(() => expect(documentService.retryOcr).toHaveBeenCalledWith('doc-1'));
    expect(await screen.findByText('OCR retry queued')).toBeInTheDocument();
  });

  it('says so when the server refuses the retry', async () => {
    documentService.retryOcr.mockImplementation(() => ok({ success: false, message: 'Already queued' }));
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'scan1.pdf');
    await user.click(within(panel).getByRole('button', { name: 'Retry OCR' }));
    expect(await screen.findByText('Could not retry OCR')).toBeInTheDocument();
  });

  it('downloads and loads history and preview by document id', async () => {
    documentService.getDocumentRetryHistory.mockImplementation(() =>
      ok({
        document_id: 'doc-1',
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
    expect(documentService.getDocumentRetryHistory).toHaveBeenCalledWith('doc-1');
    expect(documentService.view).toHaveBeenCalledWith('doc-1');
    expect(await within(panel).findByText('The file was not found or has been deleted.')).toBeInTheDocument();
    await user.click(within(panel).getByRole('button', { name: 'Download' }));
    expect(documentService.downloadFile).toHaveBeenCalledWith('doc-1', 'scan1.pdf');
  });

  it('copes with zero sizes and missing timestamps', async () => {
    serveOcr([ocrDoc('bare', { file_size: 0, last_attempt_at: null, updated_at: 'not a date', mime_type: null })]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const panel = await openDoc(user, 'bare.pdf');
    expect(within(panel).getByRole('group', { name: 'Dates' })).toHaveTextContent('—');
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('0 B');
  });
});

describe('Other import failures (read-only records, ported runtime checks)', () => {
  const record = failedDoc('rec-9', {
    failure_stage: 'ingestion',
    failure_reason: 'low_ocr_confidence',
    failure_category: 'Low OCR Confidence',
    ocr_confidence: 42.46,
    ocr_word_count: 12,
  });

  it('lists non-OCR failures with stage and reason, without selection or row actions', async () => {
    serveImports([record, failedDoc('ocr-rec', { failure_stage: 'ocr' })]);
    renderIntake(<FailedOcrPanel />);
    const grid = await importsBoard();
    expect(within(grid).getByRole('row', { name: /rec-9\.pdf/ })).toHaveTextContent('Ingestion');
    expect(within(grid).queryByRole('row', { name: /ocr-rec\.pdf/ })).not.toBeInTheDocument();
    // OCR records are never requested: every call names a non-OCR stage.
    for (const [q] of ocrService.listFailedDocuments.mock.calls) expect(q.stage).not.toBe('ocr');
    expect(ocrService.listFailedDocuments.mock.calls.every(([q]) => Boolean(q.stage))).toBe(true);
    expect(within(grid).queryAllByRole('checkbox')).toHaveLength(0);
    expect(within(grid).queryAllByRole('button')).toHaveLength(0);
  });

  it('pages across stages with the server totals: full pages, correct total, nothing dropped', async () => {
    const ingestion = Array.from({ length: 20 }, (_, i) => failedDoc(`ing-${i}`, { failure_stage: 'ingestion', failure_reason: 'file_too_large' }));
    const storage = Array.from({ length: 10 }, (_, i) => failedDoc(`sto-${i}`, { failure_stage: 'storage', failure_reason: 'other' }));
    const ocrOnes = Array.from({ length: 40 }, (_, i) => failedDoc(`ocr-${i}`, { failure_stage: 'ocr' }));
    serveImports([...ocrOnes, ...ingestion, ...storage]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await importsBoard();
    const dataRows = () => within(grid).getAllByRole('row').slice(1);
    await waitFor(() => expect(dataRows()).toHaveLength(25));
    expect(dataRows()[0]).toHaveTextContent('ing-0.pdf');
    expect(dataRows()[24]).toHaveTextContent('sto-4.pdf');
    const section = grid.closest('section') as HTMLElement;
    expect(within(section).getByText(/1–25 of 30/)).toBeInTheDocument();
    await user.click(within(section).getByRole('button', { name: /next/i }));
    await waitFor(() => expect(dataRows()).toHaveLength(5));
    expect(dataRows()[0]).toHaveTextContent('sto-5.pdf');
    expect(dataRows()[4]).toHaveTextContent('sto-9.pdf');
    expect(within(grid).queryByText('No other import failures.')).not.toBeInTheDocument();
  });

  it('shows the empty state only when the server has no non-OCR failures', async () => {
    serveImports(Array.from({ length: 30 }, (_, i) => failedDoc(`ocr-${i}`, { failure_stage: 'ocr' })));
    renderIntake(<FailedOcrPanel />);
    const grid = await importsBoard();
    expect(await within(grid).findByText('No other import failures.')).toBeInTheDocument();
    expect(within(grid.closest('section') as HTMLElement).queryByRole('button', { name: /next/i })).not.toBeInTheDocument();
  });

  it('filters by stage and reason on the server and clears the filters', async () => {
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    await importsBoard();
    await user.click(screen.getByRole('button', { name: /all stages/i }));
    expect(screen.queryByRole('option', { name: 'OCR' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('option', { name: 'Ingestion' }));
    await waitFor(() => expect(ocrService.listFailedDocuments).toHaveBeenLastCalledWith(expect.objectContaining({ stage: 'ingestion', offset: 0 })));
    await user.click(screen.getByRole('button', { name: /all reasons/i }));
    await user.click(screen.getByRole('option', { name: 'OCR timed out' }));
    await waitFor(() => expect(ocrService.listFailedDocuments).toHaveBeenLastCalledWith(expect.objectContaining({ reason: 'ocr_timeout' })));
    ocrService.listFailedDocuments.mockClear();
    await user.click(screen.getByRole('button', { name: 'Clear filters' }));
    // "All stages" asks for each non-OCR stage separately, without a reason.
    await waitFor(() => {
      const stages = ocrService.listFailedDocuments.mock.calls.map(([q]) => q.stage);
      expect(new Set(stages)).toEqual(new Set(['ingestion', 'validation', 'storage', 'processing', 'sync']));
    });
    expect(ocrService.listFailedDocuments.mock.calls.every(([q]) => q.reason === undefined)).toBe(true);
  });

  it('opens read-only details with confidence and word count and no document actions', async () => {
    serveImports([record]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await importsBoard();
    await user.click(within(grid).getByRole('rowheader', { name: /rec-9\.pdf/ }));
    const panel = await screen.findByRole('dialog', { name: 'rec-9.pdf' });
    const result = within(panel).getByRole('group', { name: 'OCR result' });
    expect(result).toHaveTextContent('42.5%');
    expect(result).toHaveTextContent('12');
    expect(within(panel).queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
    expect(within(panel).queryByRole('button', { name: 'Download' })).not.toBeInTheDocument();
    expect(ocrService.viewFailedDocument).toHaveBeenCalledWith('rec-9');
    expect(documentService.getDocumentRetryHistory).not.toHaveBeenCalled();
  });

  it.each([null, undefined])('handles a %s confidence without crashing', async (value) => {
    serveImports([{ ...record, ocr_confidence: value, ocr_word_count: value }]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    await user.click(within(await importsBoard()).getByRole('rowheader', { name: /rec-9\.pdf/ }));
    const panel = await screen.findByRole('dialog', { name: 'rec-9.pdf' });
    expect(within(panel).getByRole('group', { name: 'OCR result' })).toHaveTextContent('—');
  });

  it('falls back to the reason when the server sends no category, and handles a missing message', async () => {
    serveImports([{ ...record, failure_category: undefined, failure_reason: 'pdf_parsing_error', error_message: null }]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    const grid = await importsBoard();
    expect(within(grid).getByRole('row', { name: /rec-9\.pdf/ })).toHaveTextContent('PDF could not be read');
    await user.click(within(grid).getByRole('rowheader', { name: /rec-9\.pdf/ }));
    const panel = await screen.findByRole('dialog', { name: 'rec-9.pdf' });
    expect(within(panel).getByText('No error message was recorded.')).toBeInTheDocument();
  });

  it('copes with zero values', async () => {
    serveImports([{ ...record, file_size: 0, ocr_confidence: 0, ocr_word_count: 0, error_message: '' }]);
    const user = userEvent.setup();
    renderIntake(<FailedOcrPanel />);
    await user.click(within(await importsBoard()).getByRole('rowheader', { name: /rec-9\.pdf/ }));
    const panel = await screen.findByRole('dialog', { name: 'rec-9.pdf' });
    expect(within(panel).getByRole('group', { name: 'File' })).toHaveTextContent('0 B');
    expect(within(panel).getByRole('group', { name: 'OCR result' })).toHaveTextContent('0.0%');
  });
});
