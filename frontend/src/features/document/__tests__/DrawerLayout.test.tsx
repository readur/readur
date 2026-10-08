import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { primeApi, type ApiMock } from './mockApi';
import { makeDocument, makeOcr, renderDrawer, stubClipboard, stubObjectUrls } from './testUtils';

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

const title = () => screen.findByRole('dialog', { name: 'invoice.pdf' });
const summary = () => screen.getByRole('group', { name: 'Document summary' });
const facts = () => Array.from(summary().querySelectorAll('p > span')).map((el) => el.textContent);

beforeEach(() => {
  stubObjectUrls();
  primeApi(m);
});

describe('document drawer: facts line', () => {
  it('shows type, pages, size, source, added and OCR confidence on one line', async () => {
    load(makeDocument(), { ...makeOcr(), pages_processed: 2 } as ReturnType<typeof makeOcr>);
    renderDrawer();
    await title();
    await waitFor(() => expect(facts()).toContain('2 pages'));
    expect(facts()).toEqual(['PDF', '2 pages', '2.0 MB', 'Upload', expect.stringMatching(/^Added .*2025/), 'OCR 96%']);
    expect(within(summary()).getByText('Indexed')).toBeInTheDocument();
  });

  it('leaves out what is unknown instead of showing dashes', async () => {
    load(makeDocument({ ocr_status: 'pending', has_ocr_text: false, ocr_confidence: undefined }));
    renderDrawer();
    await title();
    expect(facts()).toEqual(['PDF', '2.0 MB', 'Upload', expect.stringMatching(/^Added /)]);
    expect(summary()).not.toHaveTextContent('—');
  });

  it('says "1 page" for a single page', async () => {
    load(makeDocument(), { ...makeOcr(), pages_processed: 1 } as ReturnType<typeof makeOcr>);
    renderDrawer();
    await title();
    await waitFor(() => expect(facts()).toContain('1 page'));
  });

  it("ignores the server's file page count, which overstates PDFs", async () => {
    // The server counts "/Type /Page" in the raw file, which also matches "/Type /Pages".
    load(makeDocument({ source_metadata: { page_count: 2, pdf_version: '1.3' } }));
    renderDrawer();
    await title();
    await screen.findByRole('region', { name: 'Extracted text' });
    expect(facts().some((f) => /page/.test(f ?? ''))).toBe(false);
    await userEvent.setup().click(screen.getByRole('tab', { name: 'Details' }));
    const details = screen.getByRole('region', { name: 'Details' });
    expect(within(details).getByText('PDF version')).toBeInTheDocument();
    expect(within(details).queryByText('Page count')).not.toBeInTheDocument();
  });

  it('shows the pages and language the OCR endpoint reports, and the sync source', async () => {
    load(
      makeDocument({ source_type: 'web_dav' }),
      { ...makeOcr(), pages_processed: 12, detected_language: 'deu' } as ReturnType<typeof makeOcr>,
    );
    renderDrawer();
    await title();
    await waitFor(() => expect(facts()).toContain('12 pages'));
    expect(facts()).toContain('DEU');
    expect(facts()).toContain('WebDAV');
  });

  it('names the connection a synced file came through', async () => {
    load(makeDocument({ source_type: 'local_folder', source_id: 'src-9' }));
    m.sourcesService.list.mockResolvedValue({ data: [{ id: 'src-9', name: 'Scanner inbox' }] });
    renderDrawer();
    await title();
    await waitFor(() => expect(facts()).toContain('Scanner inbox'));
    expect(facts()).not.toContain('Local folder');
  });

  it('keeps the source type when the connection list cannot be read', async () => {
    load(makeDocument({ source_type: 'local_folder', source_id: 'src-9' }));
    m.sourcesService.list.mockRejectedValue(new Error('forbidden'));
    renderDrawer();
    await title();
    await waitFor(() => expect(m.sourcesService.list).toHaveBeenCalled());
    expect(facts()).toContain('Local folder');
  });

  it('derives the type from the MIME type like the Library does, e.g. DOCX, never a generic "File"', async () => {
    load(makeDocument({ mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }));
    renderDrawer();
    await title();
    expect(facts()[0]).toBe('DOCX');
  });

  it('falls back to the file extension when the MIME type is generic', async () => {
    load(makeDocument({ mime_type: 'application/octet-stream', filename: 'minutes.odt', original_filename: 'minutes.odt' }));
    renderDrawer();
    await screen.findByRole('dialog', { name: 'minutes.odt' });
    expect(facts()[0]).toBe('ODT');
  });

  it.each([
    [undefined, 'Pending'],
    ['pending', 'Pending'],
    ['processing', 'OCR'],
    ['completed', 'Indexed'],
    ['failed', 'Failed'],
  ])('shows OCR status %s as %s', async (status, word) => {
    load(makeDocument({ ocr_status: status, has_ocr_text: status === 'completed' }));
    renderDrawer();
    await title();
    expect(within(summary()).getByText(word)).toBeInTheDocument();
    expect(within(summary()).queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('shows page progress while processing', async () => {
    load(makeDocument({ ocr_status: 'processing', has_ocr_text: false, ocr_progress_current: 3, ocr_progress_total: 10 }));
    renderDrawer();
    await title();
    expect(summary()).toHaveTextContent('OCR 3/10');
    expect(within(summary()).getByRole('progressbar', { name: 'OCR progress' })).toHaveAttribute('aria-valuenow', '30');
    expect(facts()).toContain('10 pages');
  });
});

const docLabels = (labels: unknown[], all: unknown[] = labels) => {
  m.labelService.getDocumentLabels.mockResolvedValue({ data: labels });
  m.labelService.list.mockResolvedValue({ data: all });
};

describe('document drawer: labels', () => {
  it('shows labels as chips with tags beside them', async () => {
    load(makeDocument({ tags: ['2025'] }));
    docLabels([tax], [tax, receipts]);
    renderDrawer();
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    expect(await within(group).findByText('Tax')).toBeInTheDocument();
    expect(within(group).getByText('2025')).toBeInTheDocument();
    expect(within(group).getByRole('button', { name: 'Edit labels' })).toBeInTheDocument();
  });

  it('offers "Add label" when there are none, saves each change at once and tells the sidebar', async () => {
    const user = userEvent.setup();
    const changed = vi.fn();
    window.addEventListener('readur:labels-changed', changed);
    load();
    docLabels([], [receipts]);
    renderDrawer();
    await title();
    const add = await screen.findByRole('button', { name: 'Add label' });
    await user.click(add);
    expect(add).toHaveAttribute('aria-expanded', 'true');
    const editor = screen.getByRole('region', { name: 'Edit labels' });
    await user.click(within(editor).getByRole('button', { name: 'Add first label' }));
    await waitFor(() => expect(m.labelService.setDocumentLabels).toHaveBeenCalledWith('doc-1', ['l-2']));
    expect(await within(screen.getByRole('group', { name: 'Labels' })).findByText('Receipts')).toBeInTheDocument();
    await waitFor(() => expect(changed).toHaveBeenCalledTimes(1));
    // Done closes the editor; nothing is left to save.
    await user.click(screen.getByRole('button', { name: 'Done' }));
    expect(screen.queryByRole('region', { name: 'Edit labels' })).not.toBeInTheDocument();
    window.removeEventListener('readur:labels-changed', changed);
  });

  it('removes a label straight from its chip, saves, and tells the list under the drawer', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    load();
    docLabels([tax, receipts]);
    renderDrawer({ list: { ids: ['doc-1'], onChanged } });
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    await user.click(await within(group).findByRole('button', { name: 'Remove Tax' }));
    await waitFor(() => expect(m.labelService.setDocumentLabels).toHaveBeenCalledWith('doc-1', ['l-2']));
    await waitFor(() => expect(within(group).queryByText('Tax')).not.toBeInTheDocument());
    expect(within(group).getByText('Receipts')).toBeInTheDocument();
    expect(onChanged).toHaveBeenCalledWith('doc-1', { labels: [receipts] });
  });

  it('puts the label back and says so when removing it fails', async () => {
    const user = userEvent.setup();
    load();
    docLabels([tax]);
    m.labelService.setDocumentLabels.mockRejectedValue(new Error('nope'));
    renderDrawer();
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    await user.click(await within(group).findByRole('button', { name: 'Remove Tax' }));
    expect(await screen.findByText("Couldn't save the labels")).toBeInTheDocument();
    expect(within(group).getByText('Tax')).toBeInTheDocument();
  });

  it('cannot remove a system label from its chip', async () => {
    load();
    docLabels([{ ...tax, is_system: true }], [tax]);
    renderDrawer();
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    await within(group).findByText('Tax');
    expect(within(group).queryByRole('button', { name: 'Remove Tax' })).not.toBeInTheDocument();
  });

  it('lets an older failed save not undo a newer change, in the drawer or the list row', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    load();
    docLabels([tax, receipts]);
    let failFirst: (e: Error) => void = () => {};
    m.labelService.setDocumentLabels
      .mockImplementationOnce(() => new Promise((_, reject) => (failFirst = reject)))
      .mockResolvedValue({ data: {} });
    renderDrawer({ list: { ids: ['doc-1'], onChanged } });
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    await user.click(await within(group).findByRole('button', { name: 'Remove Tax' }));
    await user.click(within(group).getByRole('button', { name: 'Remove Receipts' }));
    failFirst(new Error('late'));
    expect(await screen.findByText("Couldn't save the labels")).toBeInTheDocument();
    expect(within(group).queryByText('Tax')).not.toBeInTheDocument();
    expect(within(group).queryByText('Receipts')).not.toBeInTheDocument();
    expect(onChanged).toHaveBeenLastCalledWith('doc-1', { labels: [] });
  });

  it('puts the list row back to the last saved labels when the newest save fails', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    load();
    docLabels([tax, receipts]);
    let failFirst: (e: Error) => void = () => {};
    let failSecond: (e: Error) => void = () => {};
    m.labelService.setDocumentLabels
      .mockImplementationOnce(() => new Promise((_, reject) => (failFirst = reject)))
      .mockImplementationOnce(() => new Promise((_, reject) => (failSecond = reject)));
    renderDrawer({ list: { ids: ['doc-1'], onChanged } });
    await title();
    const group = screen.getByRole('group', { name: 'Labels' });
    await user.click(await within(group).findByRole('button', { name: 'Remove Tax' }));
    await user.click(within(group).getByRole('button', { name: 'Remove Receipts' }));
    failFirst(new Error('late'));
    failSecond(new Error('also'));
    // Neither save landed: the row goes back to what the server has, not to the half-way state.
    await waitFor(() => expect(onChanged).toHaveBeenLastCalledWith('doc-1', { labels: [tax, receipts] }));
    expect(await within(group).findByText('Tax')).toBeInTheDocument();
  });
});

describe('document drawer: preview and tabs', () => {
  it('keeps the file in view above the tabs, with Text chosen first', async () => {
    load();
    renderDrawer();
    const dialog = await title();
    const preview = within(dialog).getByRole('region', { name: 'Preview' });
    const tabs = within(dialog).getByRole('tablist', { name: 'About this document' });
    expect(preview.compareDocumentPosition(tabs) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(within(tabs).getAllByRole('tab').map((tab) => tab.textContent)).toEqual(['Text', 'Details', 'Comments', 'Share links']);
    expect(within(tabs).getByRole('tab', { name: 'Text' })).toHaveAttribute('aria-selected', 'true');
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toHaveTextContent('Invoice 42');
  });

  it('keeps the document actions in a bar at the bottom, after the tabs', async () => {
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    renderDrawer();
    const dialog = await title();
    const bar = within(dialog).getByRole('toolbar', { name: 'Document actions' });
    expect(within(bar).getAllByRole('button').map((b) => b.textContent || b.getAttribute('aria-label'))).toEqual([
      'Download',
      'Share',
      'Retry OCR',
      'More actions',
    ]);
    const tabs = within(dialog).getByRole('tablist', { name: 'About this document' });
    expect(tabs.compareDocumentPosition(bar) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('has a handle between the file and the tabs that resizes the file and is remembered', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer({ list: { ids: ['doc-0', 'doc-1', 'doc-2'] } });
    const dialog = await title();
    const preview = within(dialog).getByRole('region', { name: 'Preview' });
    const handle = within(dialog).getByRole('separator', { name: 'Resize preview' });
    const tabs = within(dialog).getByRole('tablist', { name: 'About this document' });
    expect(preview.compareDocumentPosition(handle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(handle.compareDocumentPosition(tabs) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const before = parseInt(preview.style.height, 10);
    act(() => handle.focus());
    await user.keyboard('{ArrowDown}');
    expect(parseInt(preview.style.height, 10)).toBe(before + 24);
    expect(window.localStorage.getItem('readur.document.previewHeight')).toBe(String(before + 24));
    // The arrow resized the file; it did not move to another document.
    expect(screen.getByRole('dialog', { name: 'invoice.pdf' })).toBeInTheDocument();
  });

  it('opens with the remembered file height', async () => {
    window.localStorage.setItem('readur.document.previewHeight', '300');
    load();
    renderDrawer();
    const dialog = await title();
    expect(within(dialog).getByRole('region', { name: 'Preview' }).style.height).toBe('300px');
  });

  it('has no resize handle when the file cannot be shown', async () => {
    load(makeDocument({ mime_type: 'application/msword' }));
    renderDrawer();
    const dialog = await title();
    expect(within(dialog).queryByRole('separator', { name: 'Resize preview' })).not.toBeInTheDocument();
  });

  it('opens the PDF with the thumbnail sidebar closed and the page fitted to the width', async () => {
    load();
    renderDrawer();
    await title();
    const frame = await screen.findByTitle('invoice.pdf');
    expect(frame.getAttribute('src')).toBe('blob:fake#navpanes=0&pagemode=none&view=FitH');
  });

  it('lets ↑/↓ scroll the text instead of changing document', async () => {
    load();
    renderDrawer();
    await title();
    expect(await screen.findByRole('region', { name: 'Extracted text' })).toHaveAttribute('data-own-arrows');
  });
});

describe('document drawer: details', () => {
  it('shows in the Details tab file, processing and activity', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load(makeDocument({ file_hash: 'a'.repeat(64), source_metadata: { camera_model: 'X100', pdf_version: '1.4' } }));
    renderDrawer();
    await title();
    expect(screen.queryByText('SHA-256')).not.toBeInTheDocument();
    await user.click(screen.getByRole('tab', { name: 'Details' }));
    const details = screen.getByRole('region', { name: 'Details' });
    expect(within(details).getByRole('heading', { name: 'File' })).toBeInTheDocument();
    expect(within(details).getByRole('heading', { name: 'Processing' })).toBeInTheDocument();
    expect(within(details).getByRole('heading', { name: 'Activity' })).toBeInTheDocument();
    // Activity older than a week shows the date, with the full time on hover.
    const added = within(details).getAllByText(/Jun 2025|Jun 15, 2025/).find((el) => el.tagName === 'TIME');
    expect(added).toHaveAttribute('dateTime', '2025-06-15T10:00:00Z');
    expect(within(details).getByText('Camera model')).toBeInTheDocument();
    expect(within(details).getByText('PDF version')).toBeInTheDocument();
    expect(within(details).getByText('X100')).toBeInTheDocument();
    await user.click(within(details).getByRole('button', { name: 'Copy hash' }));
    expect(writeText).toHaveBeenCalledWith('a'.repeat(64));
    await user.click(screen.getByRole('tab', { name: 'Text' }));
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
    renderDrawer();
    await title();
    await user.click(screen.getByRole('tab', { name: 'Details' }));
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
    renderDrawer();
    await title();
    await user.click(screen.getByRole('tab', { name: 'Details' }));
    const history = await screen.findByRole('button', { name: 'Retry history (2)' });
    expect(screen.getAllByText('Tesseract timed out').length).toBeGreaterThan(0);
    await user.click(history);
    expect(screen.getByRole('dialog', { name: 'Retry history' })).toBeInTheDocument();
  });
});
