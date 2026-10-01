import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { acknowledge } from '../../board/litStore';
import { httpError, primeApi, type ApiMock } from './mockApi';
import { makeDocument, makeOcr, renderPage, setViewport, stubObjectUrls } from './testUtils';

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

const title = () => screen.findByRole('heading', { level: 1, name: 'invoice.pdf' });

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

  it('puts the full filename in a title attribute once the h1 is clamped, and clamps it to two lines', async () => {
    // A name that fits carries no title, so assistive tech does not hear it twice.
    load();
    const first = renderPage();
    expect((await title()).querySelector('[title]')).toBeNull();
    first.unmount();
    // Clamped (content taller than the two-line box): the full name is in the title.
    const height = vi.spyOn(HTMLElement.prototype, 'scrollHeight', 'get').mockReturnValue(120);
    load();
    renderPage();
    const h1 = await title();
    expect(h1.querySelector('[title="invoice.pdf"]')).not.toBeNull();
    height.mockRestore();
    const css = readFileSync(resolve(__dirname, '../../../ui/Pass/Pass.module.css'), 'utf8');
    expect(css).toMatch(/\.clamp\s*\{[^}]*-webkit-line-clamp:\s*var\(--clamp-lines, 2\)[^}]*overflow-wrap:\s*anywhere/s);
    const header = readFileSync(resolve(__dirname, '../DocumentHeader.module.css'), 'utf8');
    expect(header).toMatch(/\.title\s*\{[^}]*text-transform:\s*none[^}]*overflow-wrap:\s*anywhere/s);
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

