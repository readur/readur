import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { acknowledge } from '../../board/litStore';
import { httpError, primeApi, type ApiMock } from './mockApi';
import { makeDocument, makeOcr, renderPage, setViewport, stubClipboard, stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());
vi.mock('../../board/litStore', () => ({ acknowledge: vi.fn() }));
vi.mock('../../../components/RetryHistoryModal', () => ({
  RetryHistoryModal: ({ open }: { open: boolean }) => (open ? <div role="dialog" aria-label="Retry history" /> : null),
}));
vi.mock('../../labels/LabelSelector', () => ({
  default: ({
    availableLabels,
    selectedLabels,
    onLabelsChange,
  }: {
    availableLabels: { id: string }[];
    selectedLabels: { id: string }[];
    onLabelsChange: (labels: unknown[]) => void;
  }) => (
    <button type="button" onClick={() => onLabelsChange([...selectedLabels, availableLabels[0]])}>
      Add first label
    </button>
  ),
}));

const m = apiModule as unknown as ApiMock;

const tax = { id: 'l-1', name: 'Tax', color: '#000000', is_system: false, created_at: '', updated_at: '' };
const receipts = { id: 'l-2', name: 'Receipts', color: '#000000', is_system: false, created_at: '', updated_at: '' };

function load(doc = makeDocument(), ocr = makeOcr()) {
  m.documentService.getById.mockResolvedValue({ data: doc });
  m.documentService.getOcrText.mockResolvedValue({ data: ocr });
}

const title = () => screen.findByRole('heading', { level: 1, name: 'invoice.pdf' });
const summary = () => screen.getByRole('group', { name: 'Document summary' });
const cell = (label: string) => {
  const term = within(summary()).getByText(label, { selector: 'dt' });
  return term.nextElementSibling as HTMLElement;
};

beforeEach(() => {
  setViewport(true);
  stubObjectUrls();
  primeApi(m);
});

describe('document page: header', () => {
  it('shows the filename as the only h1 and acknowledges the document', async () => {
    load();
    renderPage();
    expect(await title()).toBeInTheDocument();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(acknowledge).toHaveBeenCalledWith('document', 'doc-1');
  });

  it('keeps the full filename in the title attribute and clamps the h1 to two lines', async () => {
    load();
    renderPage();
    const h1 = await title();
    expect(h1.querySelector('[title="invoice.pdf"]')).not.toBeNull();
    const css = readFileSync(resolve(__dirname, '../../../ui/Pass/Pass.module.css'), 'utf8');
    expect(css).toMatch(/\.clamp\s*\{[^}]*-webkit-line-clamp:\s*var\(--clamp-lines, 2\)[^}]*overflow-wrap:\s*anywhere/s);
    const header = readFileSync(resolve(__dirname, '../../shell/PageHeader.module.css'), 'utf8');
    expect(header).toMatch(/\.title\s*\{[^}]*overflow-wrap:\s*anywhere/s);
  });

  it('links the breadcrumb back to the Library', async () => {
    load();
    renderPage();
    await title();
    const crumbs = screen.getByRole('navigation', { name: 'Breadcrumb' });
    expect(within(crumbs).getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/documents');
    expect(within(crumbs).getByText('invoice.pdf').closest('li')).toHaveAttribute('aria-current', 'page');
  });

  it('keeps the Library query the user came from', async () => {
    load();
    renderPage({ state: { from: '/documents?q=tax&sort=filename' } });
    await title();
    expect(screen.getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/documents?q=tax&sort=filename');
  });

  it('falls back to the ?q= search when there is no Library state', async () => {
    load();
    renderPage({ path: '/documents/doc-1?q=total due' });
    await title();
    expect(screen.getByRole('link', { name: 'Library' })).toHaveAttribute('href', '/documents?q=total%20due');
  });

  it('downloads the original file', async () => {
    const user = userEvent.setup();
    load();
    m.documentService.download.mockResolvedValue({ data: new Blob(['x']) });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Download' }));
    await waitFor(() => expect(m.documentService.download).toHaveBeenCalledWith('doc-1'));
    await waitFor(() => expect(click).toHaveBeenCalled());
    click.mockRestore();
  });

  it('opens the share dialog', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Share' }));
    const dialog = await screen.findByRole('dialog', { name: 'Share invoice.pdf' });
    expect(within(dialog).getByRole('button', { name: 'Create link' })).toBeInTheDocument();
    expect(m.sharedLinksService.listByDocument).toHaveBeenCalledWith('doc-1');
  });

  it('toggles the comments panel', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Comments' }));
    const panel = await screen.findByRole('dialog', { name: 'Comments' });
    expect(within(panel).getByRole('tab', { name: 'Comments', selected: true })).toBeInTheDocument();
    expect(await within(panel).findByText('No comments yet')).toBeInTheDocument();
    expect(m.commentsService.list).toHaveBeenCalledWith('doc-1');
  });
});

describe('document page: summary strip', () => {
  it('shows status, type, pages, size, source, added, language, confidence and labels', async () => {
    load();
    m.default.get.mockImplementation((url: string) =>
      Promise.resolve({ status: 200, data: url.startsWith('/labels/documents/') ? [tax] : [tax, receipts] }),
    );
    renderPage();
    await title();
    expect(cell('Status')).toHaveTextContent('INDEXED');
    expect(cell('Type')).toHaveTextContent('PDF');
    expect(cell('Pages')).toHaveTextContent('—');
    expect(cell('Size')).toHaveTextContent('2.0 MB');
    expect(cell('Source')).toHaveTextContent('Upload');
    expect(cell('Added')).toHaveTextContent('2025');
    expect(cell('Language')).toHaveTextContent('—');
    expect(cell('Confidence')).toHaveTextContent('96%');
    expect(await within(cell('Labels')).findByText('Tax')).toBeInTheDocument();
  });

  it('shows the pages and language the OCR endpoint reports, and the sync source', async () => {
    load(
      makeDocument({ source_type: 'web_dav' }),
      { ...makeOcr(), pages_processed: 12, detected_language: 'deu' } as ReturnType<typeof makeOcr>,
    );
    renderPage();
    await title();
    await waitFor(() => expect(cell('Pages')).toHaveTextContent('12'));
    expect(cell('Language')).toHaveTextContent('DEU');
    expect(cell('Source')).toHaveTextContent('WebDAV');
  });

  it('derives TYPE from the MIME type like the Library does, e.g. DOCX, never a generic "File"', async () => {
    load(
      makeDocument({
        mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        source_type: 'web_upload',
      }),
    );
    renderPage();
    await title();
    expect(cell('Type')).toHaveTextContent('DOCX');
    expect(cell('Type')).not.toHaveTextContent('File');
    expect(cell('Source')).toHaveTextContent('Upload');
  });

  it('falls back to the file extension when the MIME type is generic', async () => {
    load(makeDocument({ mime_type: 'application/octet-stream', filename: 'minutes.odt', original_filename: 'minutes.odt' }));
    renderPage();
    await screen.findByRole('heading', { level: 1, name: 'minutes.odt' });
    expect(cell('Type')).toHaveTextContent('ODT');
  });

  it('shows ADDED as a short fixed-width stamp', async () => {
    load();
    renderPage();
    await title();
    expect(cell('Added').textContent).toMatch(/\d{4}-\d{2}-\d{2} \d{2}:\d{2}/);
  });

  it.each([
    [undefined, 'PENDING'],
    ['pending', 'PENDING'],
    ['processing', 'OCR'],
    ['completed', 'INDEXED'],
    ['failed', 'FAILED'],
  ])('shows OCR status %s as %s', async (status, word) => {
    load(makeDocument({ ocr_status: status, has_ocr_text: status === 'completed' }));
    renderPage();
    await title();
    expect(cell('Status')).toHaveTextContent(new RegExp(`^.?${word}$`));
    expect(within(cell('Status')).queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('shows page progress while processing', async () => {
    load(makeDocument({ ocr_status: 'processing', has_ocr_text: false, ocr_progress_current: 3, ocr_progress_total: 10 }));
    renderPage();
    await title();
    expect(cell('Status')).toHaveTextContent('OCR 3/10');
    expect(within(cell('Status')).getByRole('progressbar', { name: 'OCR progress' })).toHaveAttribute('aria-valuenow', '30');
    expect(cell('Pages')).toHaveTextContent('10');
  });

  it('edits labels inline and saves them', async () => {
    const user = userEvent.setup();
    load();
    m.default.get.mockImplementation((url: string) =>
      Promise.resolve({ status: 200, data: url.startsWith('/labels/documents/') ? [] : [receipts] }),
    );
    renderPage();
    await title();
    const edit = screen.getByRole('button', { name: 'Edit labels' });
    await user.click(edit);
    const editor = screen.getByRole('region', { name: 'Edit labels' });
    await user.click(within(editor).getByRole('button', { name: 'Add first label' }));
    await user.click(within(editor).getByRole('button', { name: 'Save labels' }));
    await waitFor(() => expect(m.default.put).toHaveBeenCalledWith('/labels/documents/doc-1', { label_ids: ['l-2'] }));
    expect(await within(cell('Labels')).findByText('Receipts')).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Edit labels' })).not.toBeInTheDocument();
  });
});

describe('document page: retry OCR', () => {
  it('offers Retry OCR only when OCR failed', async () => {
    load(makeDocument({ ocr_status: 'completed' }));
    const { unmount } = renderPage();
    await title();
    expect(screen.queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
    unmount();

    load(makeDocument({ ocr_status: 'processing', has_ocr_text: false }));
    const second = renderPage();
    await title();
    expect(screen.queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
    second.unmount();

    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    renderPage();
    await title();
    expect(screen.getByRole('button', { name: 'Retry OCR' })).toBeEnabled();
  });

  it('queues the document again and marks the button busy meanwhile', async () => {
    const user = userEvent.setup();
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    let finish: (v: unknown) => void = () => {};
    m.documentService.bulkRetryOcr.mockReturnValue(new Promise((r) => (finish = r)));
    renderPage();
    await title();
    const retry = screen.getByRole('button', { name: 'Retry OCR' });
    await user.click(retry);
    expect(m.documentService.bulkRetryOcr).toHaveBeenCalledWith({
      mode: 'specific',
      document_ids: ['doc-1'],
      priority_override: 15,
    });
    await waitFor(() => expect(retry).toHaveAttribute('aria-busy', 'true'));
    finish({ data: {} });
    expect(await screen.findByText('OCR queued again')).toBeInTheDocument();
  });

  it('reports a failed retry', async () => {
    const user = userEvent.setup();
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    m.documentService.bulkRetryOcr.mockRejectedValue(new Error('nope'));
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Retry OCR' }));
    expect(await screen.findByText("Couldn't queue OCR again")).toBeInTheDocument();
  });
});

describe('document page: delete', () => {
  it('asks for confirmation, and Cancel keeps the document', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete this document?' });
    expect(within(dialog).getByText('invoice.pdf')).toBeInTheDocument();
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(m.documentService.delete).not.toHaveBeenCalled();
  });

  it('deletes after confirming and returns to the Library', async () => {
    const user = userEvent.setup();
    load();
    m.documentService.delete.mockResolvedValue({});
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
    expect(m.documentService.delete).toHaveBeenCalledWith('doc-1');
    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/documents');
  });

  it('keeps the page and reports when deleting fails', async () => {
    const user = userEvent.setup();
    load();
    m.documentService.delete.mockRejectedValue(new Error('nope'));
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText("Couldn't delete the document")).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'invoice.pdf' })).toBeInTheDocument();
  });

  it('offers the processed image only for images', async () => {
    const user = userEvent.setup();
    load(makeDocument({ mime_type: 'image/png', original_filename: 'invoice.pdf' }));
    m.documentService.getProcessedImage.mockResolvedValue({ data: new Blob(['png']) });
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'View processed image' }));
    const dialog = await screen.findByRole('dialog', { name: 'Processed image' });
    expect(await within(dialog).findByRole('img', { name: 'Image as prepared for OCR' })).toBeInTheDocument();
  });

  it('has no processed-image item for a PDF', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await screen.findByRole('menuitem', { name: 'Delete document' });
    expect(screen.queryByRole('menuitem', { name: 'View processed image' })).not.toBeInTheDocument();
  });
});

describe('document page: states', () => {
  it('shows skeletons while loading', () => {
    const never = new Promise(() => {});
    m.documentService.getById.mockReturnValue(never);
    m.default.get.mockReturnValue(never);
    m.documentService.getDocumentRetryHistory.mockReturnValue(never);
    renderPage();
    expect(screen.getByRole('status', { name: 'Loading document' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { level: 1 })).not.toBeInTheDocument();
  });

  it('shows "Document not found" with a way back on a 404', async () => {
    m.documentService.getById.mockRejectedValue(httpError(404));
    renderPage();
    expect(await screen.findByRole('heading', { level: 1, name: 'Document not found' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Back to Library' })).toHaveAttribute('href', '/documents');
    expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
  });

  it('shows a retryable error for other failures', async () => {
    const user = userEvent.setup();
    m.documentService.getById.mockRejectedValueOnce(new Error('Network down'));
    m.documentService.getById.mockResolvedValue({ data: makeDocument() });
    m.documentService.getOcrText.mockResolvedValue({ data: makeOcr() });
    renderPage();
    expect(await screen.findByRole('heading', { level: 1, name: "Couldn't load this document" })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await title()).toBeInTheDocument();
    expect(m.documentService.getById).toHaveBeenCalledTimes(2);
  });
});

describe('document page: layout and details', () => {
  it('shows preview and text side by side on a wide screen', async () => {
    load();
    renderPage();
    await title();
    expect(screen.getByRole('region', { name: 'Preview' })).toBeInTheDocument();
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toHaveTextContent('Invoice 42');
    expect(screen.queryByRole('tablist', { name: 'Document views' })).not.toBeInTheDocument();
  });

  it('uses Preview / Text / Details tabs on a narrow screen', async () => {
    const user = userEvent.setup();
    setViewport(false);
    load();
    renderPage();
    await title();
    const tabs = screen.getByRole('tablist', { name: 'Document views' });
    expect(within(tabs).getAllByRole('tab').map((t) => t.textContent)).toEqual(['Preview', 'Text', 'Details']);
    expect(within(tabs).getByRole('tab', { name: 'Preview' })).toHaveAttribute('aria-selected', 'true');
    await user.click(within(tabs).getByRole('tab', { name: 'Text' }));
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toHaveTextContent('Total due');
    await user.click(within(tabs).getByRole('tab', { name: 'Details' }));
    expect(screen.getByText('SHA-256')).toBeInTheDocument();
  });

  it('opens on the Text tab when arriving from a search', async () => {
    setViewport(false);
    load();
    renderPage({ path: '/documents/doc-1?q=invoice' });
    await title();
    expect(screen.getByRole('tab', { name: 'Text' })).toHaveAttribute('aria-selected', 'true');
  });

  it('keeps details collapsed until asked, then shows file, processing and activity', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load(makeDocument({ file_hash: 'a'.repeat(64), source_metadata: { camera_model: 'X100' } }));
    renderPage();
    await title();
    const toggle = screen.getByRole('button', { name: 'Show details' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('SHA-256')).not.toBeInTheDocument();
    await user.click(toggle);
    expect(screen.getByRole('button', { name: 'Hide details' })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('heading', { name: 'File' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Processing' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Activity' })).toBeInTheDocument();
    expect(screen.getByText('Camera Model')).toBeInTheDocument();
    expect(screen.getByText('X100')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Copy hash' }));
    expect(writeText).toHaveBeenCalledWith('a'.repeat(64));
  });

  it('shows retries and the last failure from the retry history', async () => {
    const user = userEvent.setup();
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    m.documentService.getDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'doc-1',
        total_retries: 2,
        retry_history: [
          { id: 'r1', retry_reason: 'manual', priority: 10, created_at: '2025-06-15T11:00:00Z', previous_error: 'old error' },
          { id: 'r2', retry_reason: 'manual', priority: 10, created_at: '2025-06-16T11:00:00Z', previous_error: 'Tesseract timed out' },
        ],
      },
    });
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Show details' }));
    const history = await screen.findByRole('button', { name: 'Retry history (2)' });
    expect(screen.getAllByText('Tesseract timed out').length).toBeGreaterThan(0);
    await user.click(history);
    expect(screen.getByRole('dialog', { name: 'Retry history' })).toBeInTheDocument();
  });
});
