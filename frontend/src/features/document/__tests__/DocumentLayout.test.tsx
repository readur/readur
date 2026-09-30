import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { primeApi, type ApiMock } from './mockApi';
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
const facts = () => Array.from(summary().querySelectorAll('p > span')).map((el) => el.textContent);
const viewSwitch = () => screen.getByRole('radiogroup', { name: 'View' });

beforeEach(() => {
  setViewport(true);
  stubObjectUrls();
  primeApi(m);
});

describe('document page: facts line', () => {
  it('shows type, pages, size, source, added and OCR confidence on one line', async () => {
    load(makeDocument({ source_metadata: { page_count: 2 } }));
    renderPage();
    await title();
    expect(facts()).toEqual(['PDF', '2 pages', '2.0 MB', 'Upload', expect.stringMatching(/^Added .*2025/), 'OCR 96%']);
    expect(within(summary()).getByText('INDEXED')).toBeInTheDocument();
  });

  it('leaves out what is unknown instead of showing dashes', async () => {
    load(makeDocument({ ocr_status: 'pending', has_ocr_text: false, ocr_confidence: undefined }));
    renderPage();
    await title();
    expect(facts()).toEqual(['PDF', '2.0 MB', 'Upload', expect.stringMatching(/^Added /)]);
    expect(summary()).not.toHaveTextContent('—');
  });

  it('says "1 page" for a single page', async () => {
    load(makeDocument({ source_metadata: { page_count: 1 } }));
    renderPage();
    await title();
    expect(facts()).toContain('1 page');
  });

  it('shows the pages and language the OCR endpoint reports, and the sync source', async () => {
    load(
      makeDocument({ source_type: 'web_dav' }),
      { ...makeOcr(), pages_processed: 12, detected_language: 'deu' } as ReturnType<typeof makeOcr>,
    );
    renderPage();
    await title();
    await waitFor(() => expect(facts()).toContain('12 pages'));
    expect(facts()).toContain('DEU');
    expect(facts()).toContain('WebDAV');
  });

  it('names the connection a synced file came through', async () => {
    load(makeDocument({ source_type: 'local_folder', source_id: 'src-9' }));
    m.sourcesService.list.mockResolvedValue({ data: [{ id: 'src-9', name: 'Scanner inbox' }] });
    renderPage();
    await title();
    await waitFor(() => expect(facts()).toContain('Scanner inbox'));
    expect(facts()).not.toContain('Local folder');
  });

  it('keeps the source type when the connection list cannot be read', async () => {
    load(makeDocument({ source_type: 'local_folder', source_id: 'src-9' }));
    m.sourcesService.list.mockRejectedValue(new Error('forbidden'));
    renderPage();
    await title();
    await waitFor(() => expect(m.sourcesService.list).toHaveBeenCalled());
    expect(facts()).toContain('Local folder');
  });

  it('derives the type from the MIME type like the Library does, e.g. DOCX, never a generic "File"', async () => {
    load(makeDocument({ mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }));
    renderPage();
    await title();
    expect(facts()[0]).toBe('DOCX');
  });

  it('falls back to the file extension when the MIME type is generic', async () => {
    load(makeDocument({ mime_type: 'application/octet-stream', filename: 'minutes.odt', original_filename: 'minutes.odt' }));
    renderPage();
    await screen.findByRole('heading', { level: 1, name: 'minutes.odt' });
    expect(facts()[0]).toBe('ODT');
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
    expect(within(summary()).getByText(word)).toBeInTheDocument();
    expect(within(summary()).queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('shows page progress while processing', async () => {
    load(makeDocument({ ocr_status: 'processing', has_ocr_text: false, ocr_progress_current: 3, ocr_progress_total: 10 }));
    renderPage();
    await title();
    expect(summary()).toHaveTextContent('OCR 3/10');
    expect(within(summary()).getByRole('progressbar', { name: 'OCR progress' })).toHaveAttribute('aria-valuenow', '30');
    expect(facts()).toContain('10 pages');
  });
});

describe('document page: labels', () => {
  it('shows labels as chips with tags beside them', async () => {
    load(makeDocument({ tags: ['2025'] }));
    m.default.get.mockImplementation((url: string) =>
      Promise.resolve({ status: 200, data: url.startsWith('/labels/documents/') ? [tax] : [tax, receipts] }),
    );
    renderPage();
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    expect(await within(group).findByText('Tax')).toBeInTheDocument();
    expect(within(group).getByText('2025')).toBeInTheDocument();
    expect(within(group).getByRole('button', { name: 'Edit labels' })).toBeInTheDocument();
  });

  it('offers "Add label" when there are none, edits inline, saves and tells the sidebar', async () => {
    const user = userEvent.setup();
    const changed = vi.fn();
    window.addEventListener('readur:labels-changed', changed);
    load();
    m.default.get.mockImplementation((url: string) =>
      Promise.resolve({ status: 200, data: url.startsWith('/labels/documents/') ? [] : [receipts] }),
    );
    renderPage();
    await title();
    const add = screen.getByRole('button', { name: 'Add label' });
    await user.click(add);
    expect(add).toHaveAttribute('aria-expanded', 'true');
    const editor = screen.getByRole('region', { name: 'Edit labels' });
    await user.click(within(editor).getByRole('button', { name: 'Add first label' }));
    await user.click(within(editor).getByRole('button', { name: 'Save labels' }));
    await waitFor(() => expect(m.default.put).toHaveBeenCalledWith('/labels/documents/doc-1', { label_ids: ['l-2'] }));
    expect(await within(screen.getByRole('group', { name: 'Labels' })).findByText('Receipts')).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Edit labels' })).not.toBeInTheDocument();
    expect(changed).toHaveBeenCalledTimes(1);
    window.removeEventListener('readur:labels-changed', changed);
  });

  it('cancels editing without saving', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Add label' }));
    await user.click(within(screen.getByRole('region', { name: 'Edit labels' })).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('region', { name: 'Edit labels' })).not.toBeInTheDocument();
    expect(m.default.put).not.toHaveBeenCalled();
  });

  it('reports labels that could not be saved and keeps the editor open', async () => {
    const user = userEvent.setup();
    load();
    m.default.get.mockImplementation((url: string) =>
      Promise.resolve({ status: 200, data: url.startsWith('/labels/documents/') ? [] : [receipts] }),
    );
    m.default.put.mockRejectedValue(new Error('nope'));
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Add label' }));
    const editor = screen.getByRole('region', { name: 'Edit labels' });
    await user.click(within(editor).getByRole('button', { name: 'Add first label' }));
    await user.click(within(editor).getByRole('button', { name: 'Save labels' }));
    expect(await screen.findByText("Couldn't save the labels")).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Edit labels' })).toBeInTheDocument();
  });
});

describe('document page: views', () => {
  it('shows the file and the text side by side on a wide screen', async () => {
    load();
    renderPage();
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Side by side' })).toBeChecked();
    expect(screen.getByRole('region', { name: 'Document' })).toBeVisible();
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toHaveTextContent('Invoice 42');
  });

  it('opens the PDF with the thumbnail sidebar closed and the page fitted to the width', async () => {
    load();
    renderPage();
    await title();
    const frame = await screen.findByTitle('invoice.pdf');
    expect(frame.getAttribute('src')).toBe('blob:fake#navpanes=0&pagemode=none&view=FitH');
  });

  it('defaults to Document where two panes fit but side by side is not the default', async () => {
    setViewport('mid');
    load();
    renderPage();
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Document' })).toBeChecked();
    expect(screen.queryByRole('region', { name: 'Extracted text' })).not.toBeInTheDocument();
  });

  it('offers only Document and Text on a phone', async () => {
    setViewport(false);
    load();
    renderPage();
    await title();
    expect(within(viewSwitch()).getAllByRole('radio').map((r) => r.textContent)).toEqual(['Document', 'Text']);
    expect(within(viewSwitch()).getByRole('radio', { name: 'Document' })).toBeChecked();
  });

  it('switches views and remembers the choice', async () => {
    const user = userEvent.setup();
    load();
    const first = renderPage();
    await title();
    await user.click(within(viewSwitch()).getByRole('radio', { name: 'Text' }));
    expect(screen.queryByRole('region', { name: 'Document' })).not.toBeInTheDocument();
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toBeVisible();
    expect(window.localStorage.getItem('readur.document.view')).toBe('text');
    first.unmount();

    load();
    renderPage();
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Text' })).toBeChecked();
  });

  it('falls back to Document when a remembered Side by side does not fit', async () => {
    window.localStorage.setItem('readur.document.view', 'split');
    setViewport(false);
    load();
    renderPage();
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Document' })).toBeChecked();
  });

  it('brings the text into view when arriving from a search on a phone', async () => {
    setViewport(false);
    load();
    renderPage({ path: '/documents/doc-1?q=invoice' });
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Text' })).toBeChecked();
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toBeVisible();
  });

  it('shows side by side when arriving from a search on a mid-size screen', async () => {
    setViewport('mid');
    load();
    renderPage({ path: '/documents/doc-1?q=invoice' });
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Side by side' })).toBeChecked();
  });

  it('opens a file the browser cannot show on its text', async () => {
    setViewport('mid');
    load(makeDocument({ mime_type: 'application/msword' }));
    renderPage();
    await title();
    expect(within(viewSwitch()).getByRole('radio', { name: 'Text' })).toBeChecked();
  });
});

describe('document page: details', () => {
  it('keeps details closed until asked, then shows file, processing and activity', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load(makeDocument({ file_hash: 'a'.repeat(64), source_metadata: { camera_model: 'X100', pdf_version: '1.4' } }));
    renderPage();
    await title();
    const toggle = screen.getByRole('button', { name: 'Details' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('SHA-256')).not.toBeInTheDocument();
    await user.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    const details = screen.getByRole('region', { name: 'Details' });
    expect(toggle).toHaveAttribute('aria-controls', details.id);
    expect(within(details).getByRole('heading', { name: 'File' })).toBeInTheDocument();
    expect(within(details).getByRole('heading', { name: 'Processing' })).toBeInTheDocument();
    expect(within(details).getByRole('heading', { name: 'Activity' })).toBeInTheDocument();
    expect(within(details).getByText('Camera model')).toBeInTheDocument();
    expect(within(details).getByText('PDF version')).toBeInTheDocument();
    expect(within(details).getByText('X100')).toBeInTheDocument();
    await user.click(within(details).getByRole('button', { name: 'Copy hash' }));
    expect(writeText).toHaveBeenCalledWith('a'.repeat(64));
    await user.click(toggle);
    expect(screen.queryByRole('region', { name: 'Details' })).not.toBeInTheDocument();
  });

  it('shows where a synced file came from and its file-system facts', async () => {
    const user = userEvent.setup();
    load(
      makeDocument({
        source_type: 'webdav',
        source_path: '/remote/scans/invoice.pdf',
        source_id: 'src-1',
        original_created_at: '2025-01-02T09:00:00Z',
        original_modified_at: '2025-01-03T09:00:00Z',
        file_owner: 'scanner',
        file_group: 'office',
        file_permissions: 420,
      }),
    );
    renderPage();
    await title();
    await user.click(screen.getByRole('button', { name: 'Details' }));
    const details = screen.getByRole('region', { name: 'Details' });
    expect(within(details).getByRole('heading', { name: 'Source' })).toBeInTheDocument();
    expect(within(details).getByText('/remote/scans/invoice.pdf')).toBeInTheDocument();
    expect(within(details).getByText('src-1')).toBeInTheDocument();
    expect(within(details).getByText('Originally created')).toBeInTheDocument();
    expect(within(details).getByText('Originally modified')).toBeInTheDocument();
    expect(within(details).getByText('scanner')).toBeInTheDocument();
    expect(within(details).getByText('office')).toBeInTheDocument();
    expect(within(details).getByText('644 (420)')).toBeInTheDocument();
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
    await user.click(screen.getByRole('button', { name: 'Details' }));
    const history = await screen.findByRole('button', { name: 'Retry history (2)' });
    expect(screen.getAllByText('Tesseract timed out').length).toBeGreaterThan(0);
    await user.click(history);
    expect(screen.getByRole('dialog', { name: 'Retry history' })).toBeInTheDocument();
  });
});
