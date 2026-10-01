import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { ToastProvider } from '../../../ui';
import SharedRoute from '../SharedRoute';
import { httpError, type ApiMock } from './mockApi';
import { stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());

const m = apiModule as unknown as ApiMock;

const metadata = {
  filename: 'x.pdf',
  original_filename: 'contract.pdf',
  file_size: 1536,
  mime_type: 'application/pdf',
  requires_password: false,
  created_at: '2025-06-15T10:00:00Z',
};

function renderShared(path = '/shared/tok-1') {
  return render(
    <ToastProvider>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/shared/:token" element={<SharedRoute />} />
        </Routes>
      </MemoryRouter>
    </ToastProvider>,
  );
}

beforeEach(() => {
  stubObjectUrls();
});

describe('shared document page', () => {
  it('shows a minimal header and a loading state', () => {
    m.sharedLinksPublicService.getMetadata.mockReturnValue(new Promise(() => {}));
    renderShared();
    expect(screen.getByRole('banner')).toHaveTextContent('Readur');
    // Wordmark only: no label above or beside it.
    expect(screen.getByRole('banner')).not.toHaveTextContent('Shared document');
    expect(screen.getByRole('status', { name: 'Loading shared document' })).toBeInTheDocument();
    expect(screen.queryByRole('navigation')).not.toBeInTheDocument();
  });

  it('shows the file facts and downloads an open link', async () => {
    const user = userEvent.setup();
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: metadata });
    m.sharedLinksPublicService.downloadDocument.mockResolvedValue({
      data: new Blob(['pdf']),
      headers: { 'content-disposition': 'attachment; filename="contract.pdf"' },
    });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderShared();
    expect(await screen.findByRole('heading', { level: 1, name: 'contract.pdf' })).toBeInTheDocument();
    const facts = screen.getByRole('group', { name: 'Document summary' });
    expect(facts).toHaveTextContent('PDF');
    expect(facts).toHaveTextContent('1.5 KB');
    await user.click(screen.getByRole('button', { name: 'Download' }));
    expect(m.sharedLinksPublicService.downloadDocument).toHaveBeenCalledWith('tok-1', undefined);
    await waitFor(() => expect(click).toHaveBeenCalled());
    click.mockRestore();
  });

  it.each([
    ['report.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'DOCX'],
    ['scan.tiff', 'image/tiff', 'TIFF'],
    ['notes.md', 'application/octet-stream', 'MD'],
  ])('shows %s as a short type code, never a raw MIME type', async (name, mime, code) => {
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: { ...metadata, original_filename: name, mime_type: mime } });
    renderShared();
    await screen.findByRole('heading', { level: 1, name });
    const facts = screen.getByRole('group', { name: 'Document summary' });
    expect(facts.firstElementChild).toHaveTextContent(code);
    expect(facts).not.toHaveTextContent(mime);
  });

  it('says when the link expires, or that it never does', async () => {
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: { ...metadata, expires_at: '2030-01-02T10:00:00Z' } });
    const first = renderShared();
    await screen.findByRole('heading', { level: 1, name: 'contract.pdf' });
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent(/Link expires .*2030/);
    first.unmount();

    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: { ...metadata, expires_at: null } });
    renderShared();
    await screen.findByRole('heading', { level: 1, name: 'contract.pdf' });
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent('Link never expires');
  });

  it('loads the preview only when asked, since it counts as a view', async () => {
    const user = userEvent.setup();
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: { ...metadata, mime_type: 'image/png' } });
    m.sharedLinksPublicService.viewDocument.mockResolvedValue({ data: new Blob(['png']) });
    renderShared();
    await screen.findByRole('heading', { level: 1, name: 'contract.pdf' });
    expect(m.sharedLinksPublicService.viewDocument).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Show preview' }));
    expect(await screen.findByRole('img', { name: 'contract.pdf' })).toHaveAttribute('src', 'blob:fake');
    expect(m.sharedLinksPublicService.viewDocument).toHaveBeenCalledWith('tok-1', undefined);
  });

  it('asks for a password, rejects a wrong one, and opens with the right one', async () => {
    const user = userEvent.setup();
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: { ...metadata, requires_password: true } });
    m.sharedLinksPublicService.verifyPassword
      .mockResolvedValueOnce({ data: { valid: false } })
      .mockResolvedValueOnce({ data: { valid: true } });
    m.sharedLinksPublicService.downloadDocument.mockResolvedValue({ data: new Blob(['pdf']), headers: {} });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderShared();

    expect(await screen.findByRole('heading', { level: 1, name: 'Password required' })).toBeInTheDocument();
    const field = screen.getByLabelText('Password');
    const submit = screen.getByRole('button', { name: 'Open document' });
    expect(submit).toBeDisabled();
    await user.type(field, 'wrong');
    await user.click(submit);
    expect(m.sharedLinksPublicService.verifyPassword).toHaveBeenCalledWith('tok-1', 'wrong');
    expect(await screen.findByText('That password is not right. Try again.')).toBeInTheDocument();
    expect(field).toHaveAttribute('aria-invalid', 'true');

    await user.clear(field);
    await user.type(field, 'right');
    await user.click(submit);
    expect(await screen.findByRole('heading', { level: 1, name: 'contract.pdf' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Download' }));
    expect(m.sharedLinksPublicService.downloadDocument).toHaveBeenCalledWith('tok-1', 'right');
    click.mockRestore();
  });

  it('shows the expired state for a 410', async () => {
    m.sharedLinksPublicService.getMetadata.mockRejectedValue(httpError(410));
    renderShared();
    expect(await screen.findByRole('heading', { level: 1, name: 'This link has expired' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Download' })).not.toBeInTheDocument();
  });

  it('shows the expired state when a download hits the view limit', async () => {
    const user = userEvent.setup();
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: metadata });
    m.sharedLinksPublicService.downloadDocument.mockRejectedValue(httpError(410));
    renderShared();
    await screen.findByRole('heading', { level: 1, name: 'contract.pdf' });
    await user.click(screen.getByRole('button', { name: 'Download' }));
    expect(await screen.findByRole('heading', { level: 1, name: 'This link has expired' })).toBeInTheDocument();
  });

  it('explains a missing link and other errors', async () => {
    m.sharedLinksPublicService.getMetadata.mockRejectedValue(httpError(404));
    const { unmount } = renderShared();
    expect(await screen.findByRole('heading', { level: 1, name: 'This link is not available' })).toBeInTheDocument();
    expect(screen.getByText('It does not exist or has been removed.')).toBeInTheDocument();
    unmount();

    m.sharedLinksPublicService.getMetadata.mockRejectedValue(httpError(500));
    renderShared();
    expect(await screen.findByText("The shared document couldn't be loaded. Try again later.")).toBeInTheDocument();
  });

  it('reports a failed download without leaving the page', async () => {
    const user = userEvent.setup();
    m.sharedLinksPublicService.getMetadata.mockResolvedValue({ data: metadata });
    m.sharedLinksPublicService.downloadDocument.mockRejectedValue(httpError(500));
    renderShared();
    await screen.findByRole('heading', { level: 1, name: 'contract.pdf' });
    await user.click(screen.getByRole('button', { name: 'Download' }));
    expect(await screen.findByRole('alert')).toHaveTextContent("Couldn't download the file. Try again.");
  });
});
