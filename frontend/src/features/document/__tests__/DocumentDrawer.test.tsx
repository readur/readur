import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { acknowledge } from '../../board/litStore';
import { httpError, primeApi, type ApiMock } from './mockApi';
import { DOCUMENTS_CHANGED_EVENT } from '../../shell/useDocumentTotal';
import { makeDocument, makeOcr, renderDrawer, stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());
vi.mock('../../board/litStore', () => ({ acknowledge: vi.fn() }));
vi.mock('../../../components/RetryHistoryModal', () => ({
  RetryHistoryModal: ({ open }: { open: boolean }) => (open ? <div role="dialog" aria-label="Retry history" /> : null),
}));
vi.mock('../../labels/LabelSelector', () => ({ default: () => null }));

const m = apiModule as unknown as ApiMock;

function load(doc = makeDocument(), ocr = makeOcr()) {
  m.documentService.getById.mockResolvedValue({ data: doc });
  m.documentService.getOcrText.mockResolvedValue({ data: ocr });
}

const title = () => screen.findByRole('dialog', { name: 'invoice.pdf' });
const url = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;

beforeEach(() => {
  stubObjectUrls();
  primeApi(m);
});

describe('document drawer: head', () => {
  it('is a dialog named by the filename over the page, and acknowledges the document', async () => {
    load();
    renderDrawer({ path: '/home?document=doc-1' });
    expect(await title()).toBeInTheDocument();
    expect(url()).toBe('/home?document=doc-1');
    expect(acknowledge).toHaveBeenCalledWith('document', 'doc-1');
  });

  it('titles the drawer from the list while the document loads', async () => {
    m.documentService.getById.mockReturnValue(new Promise(() => {}));
    renderDrawer({ list: { ids: ['doc-1'], peek: () => ({ name: 'from-the-row.pdf' }) } });
    expect(await screen.findByRole('dialog', { name: 'from-the-row.pdf' })).toBeInTheDocument();
  });

  it('closes by removing ?document= and leaves the page as it was', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer({ path: '/documents?labels=work&document=doc-1' });
    await title();
    await user.click(screen.getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(url()).toBe('/documents?labels=work'));
  });

  it('steps to the next and previous document of the list with ↓/↑', async () => {
    const user = userEvent.setup();
    m.documentService.getById.mockImplementation((id: string) =>
      Promise.resolve({ data: makeDocument({ id, original_filename: `${id}.pdf` }) }),
    );
    m.documentService.getOcrText.mockResolvedValue({ data: makeOcr() });
    renderDrawer({ path: '/documents?document=doc-2', list: { ids: ['doc-1', 'doc-2', 'doc-3'] } });
    await screen.findByRole('dialog', { name: 'doc-2.pdf' });
    await user.click(screen.getByRole('button', { name: 'Close' }).closest('header') as HTMLElement);
    await user.keyboard('{ArrowDown}');
    expect(await screen.findByRole('dialog', { name: 'doc-3.pdf' })).toBeInTheDocument();
    expect(url()).toBe('/documents?document=doc-3');
    await user.keyboard('{ArrowUp}{ArrowUp}');
    expect(await screen.findByRole('dialog', { name: 'doc-1.pdf' })).toBeInTheDocument();
  });

  it('downloads the original file', async () => {
    const user = userEvent.setup();
    load();
    m.documentService.download.mockResolvedValue({ data: new Blob(['x']) });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'Download' }));
    await waitFor(() => expect(m.documentService.download).toHaveBeenCalledWith('doc-1'));
    await waitFor(() => expect(click).toHaveBeenCalled());
    click.mockRestore();
  });

  it('Share opens the Share links tab', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'Share' }));
    const panel = await screen.findByRole('tabpanel', { name: 'Share links' });
    expect(await within(panel).findByRole('button', { name: 'Create link' })).toBeInTheDocument();
    expect(m.sharedLinksService.listByDocument).toHaveBeenCalledWith('doc-1');
  });

  it('shows the comment count on its tab and the comments inside it', async () => {
    const user = userEvent.setup();
    load();
    m.commentsService.list.mockResolvedValue({
      data: [{ id: 'c1', document_id: 'doc-1', user_id: 'u', parent_id: null, content: 'Check page 4', is_edited: false, created_at: '', updated_at: '', username: 'ada', user_role: 'user', reply_count: 1, replies: [] }],
    });
    renderDrawer();
    await title();
    const tab = await screen.findByRole('tab', { name: 'Comments 2' });
    await user.click(tab);
    expect(await within(screen.getByRole('tabpanel', { name: /Comments/ })).findByText('Check page 4')).toBeInTheDocument();
    expect(m.commentsService.list).toHaveBeenCalledWith('doc-1');
  });
});

describe('document drawer: retry OCR', () => {
  it('offers Retry OCR only when OCR failed', async () => {
    load(makeDocument({ ocr_status: 'completed' }));
    const { unmount } = renderDrawer();
    await title();
    expect(screen.queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
    unmount();

    load(makeDocument({ ocr_status: 'processing', has_ocr_text: false }));
    const second = renderDrawer();
    await title();
    expect(screen.queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
    second.unmount();

    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    renderDrawer();
    await title();
    expect(screen.getByRole('button', { name: 'Retry OCR' })).toBeEnabled();
  });

  it('queues the document again and marks the button busy meanwhile', async () => {
    const user = userEvent.setup();
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    let finish: (v: unknown) => void = () => {};
    m.documentService.retryOcr.mockReturnValue(new Promise((r) => (finish = r)));
    renderDrawer();
    await title();
    const retry = screen.getByRole('button', { name: 'Retry OCR' });
    await user.click(retry);
    expect(m.documentService.retryOcr).toHaveBeenCalledWith('doc-1');
    await waitFor(() => expect(retry).toHaveAttribute('aria-busy', 'true'));
    finish({ data: {} });
    expect(await screen.findByText('OCR queued again')).toBeInTheDocument();
  });

  it('reports a failed retry', async () => {
    const user = userEvent.setup();
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    m.documentService.retryOcr.mockRejectedValue(new Error('nope'));
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'Retry OCR' }));
    expect(await screen.findByText("Couldn't queue OCR again")).toBeInTheDocument();
  });
});

describe('document drawer: delete', () => {
  it('asks for confirmation, and Cancel keeps the document', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete this document?' });
    expect(within(dialog).getByText('invoice.pdf')).toBeInTheDocument();
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(m.documentService.delete).not.toHaveBeenCalled();
  });

  it('deletes after confirming, closes the drawer and tells the lists', async () => {
    const user = userEvent.setup();
    const onDeleted = vi.fn();
    const changed = vi.fn();
    window.addEventListener(DOCUMENTS_CHANGED_EVENT, changed);
    load();
    m.documentService.delete.mockResolvedValue({});
    renderDrawer({ path: '/documents?labels=work&document=doc-1', list: { ids: ['doc-1'], onDeleted } });
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
    expect(m.documentService.delete).toHaveBeenCalledWith('doc-1');
    await waitFor(() => expect(url()).toBe('/documents?labels=work'));
    expect(onDeleted).toHaveBeenCalledWith('doc-1');
    expect(changed).toHaveBeenCalledTimes(1);
    window.removeEventListener(DOCUMENTS_CHANGED_EVENT, changed);
  });

  it('stays open and reports when deleting fails', async () => {
    const user = userEvent.setup();
    load();
    m.documentService.delete.mockRejectedValue(new Error('nope'));
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText("Couldn't delete the document")).toBeInTheDocument();
    expect(screen.getByRole('dialog', { name: 'invoice.pdf' })).toBeInTheDocument();
  });

  it('offers the processed image only for images', async () => {
    const user = userEvent.setup();
    load(makeDocument({ mime_type: 'image/png', original_filename: 'invoice.pdf' }));
    m.documentService.getProcessedImage.mockResolvedValue({ data: new Blob(['png']) });
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await user.click(await screen.findByRole('menuitem', { name: 'View processed image' }));
    const dialog = await screen.findByRole('dialog', { name: 'Processed image' });
    expect(await within(dialog).findByRole('img', { name: 'Image as prepared for OCR' })).toBeInTheDocument();
  });

  it('has no processed-image item for a PDF', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    await user.click(screen.getByRole('button', { name: 'More actions' }));
    await screen.findByRole('menuitem', { name: 'Delete document' });
    expect(screen.queryByRole('menuitem', { name: 'View processed image' })).not.toBeInTheDocument();
  });
});

describe('document drawer: states', () => {
  it('shows skeletons while loading', async () => {
    const never = new Promise(() => {});
    m.documentService.getById.mockReturnValue(never);
    m.default.get.mockReturnValue(never);
    m.documentService.getDocumentRetryHistory.mockReturnValue(never);
    renderDrawer();
    expect(await screen.findByRole('status', { name: 'Loading document' })).toBeInTheDocument();
  });

  it('says "Document not found" on a 404, and Close removes ?document=', async () => {
    const user = userEvent.setup();
    m.documentService.getById.mockRejectedValue(httpError(404));
    renderDrawer({ path: '/home?document=doc-1' });
    expect(await screen.findByRole('heading', { name: 'Document not found' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Try again' })).not.toBeInTheDocument();
    const dialog = screen.getByRole('dialog');
    await user.click(within(dialog).getAllByRole('button', { name: 'Close' }).at(-1) as HTMLElement);
    await waitFor(() => expect(url()).toBe('/home'));
  });

  it('shows a retryable error for other failures', async () => {
    const user = userEvent.setup();
    m.documentService.getById.mockRejectedValueOnce(new Error('Network down'));
    m.documentService.getById.mockResolvedValue({ data: makeDocument() });
    m.documentService.getOcrText.mockResolvedValue({ data: makeOcr() });
    renderDrawer();
    expect(await screen.findByRole('heading', { name: "Couldn't load this document" })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await title()).toBeInTheDocument();
    expect(m.documentService.getById).toHaveBeenCalledTimes(2);
  });
});

