import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import type { SharedLinkData } from '../../../services/api';
import { SharedLinksDialog } from '../sharing/SharedLinksDialog';
import { httpError, type ApiMock } from './mockApi';
import { Providers, stubClipboard } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());

const m = apiModule as unknown as ApiMock;

const link = (overrides: Partial<SharedLinkData> = {}): SharedLinkData => ({
  id: 'link-1',
  document_id: 'doc-1',
  token: 'abcdef1234567890wxyz',
  url: 'https://readur.example/shared/abcdef1234567890wxyz',
  has_password: false,
  expires_at: null,
  max_views: null,
  view_count: 3,
  is_expired: false,
  is_revoked: false,
  created_at: '2025-06-15T10:00:00Z',
  ...overrides,
});

function Harness() {
  const [open, setOpen] = useState(true);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open share
      </button>
      <SharedLinksDialog documentId="doc-1" filename="invoice.pdf" isOpen={open} onOpenChange={setOpen} />
    </>
  );
}

function renderDialog() {
  render(
    <Providers>
      <Harness />
    </Providers>,
  );
  return screen.findByRole('dialog', { name: 'Share invoice.pdf' });
}

beforeEach(() => {
  m.sharedLinksService.listByDocument.mockResolvedValue({ data: [] });
});

describe('share links dialog', () => {
  it('creates an open link and shows its URL to copy', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    const created = link({ id: 'link-9' });
    m.sharedLinksService.create.mockResolvedValue({ data: created });
    const dialog = await renderDialog();
    expect(await within(dialog).findByText('No share links yet.')).toBeInTheDocument();
    m.sharedLinksService.listByDocument.mockResolvedValue({ data: [created] });

    await user.click(within(dialog).getByRole('button', { name: 'Create link' }));
    expect(m.sharedLinksService.create).toHaveBeenCalledWith({ document_id: 'doc-1' });
    expect(await within(dialog).findByRole('textbox', { name: 'Share link' })).toHaveValue(created.url);
    await user.click(within(dialog).getAllByRole('button', { name: 'Copy link' })[0]);
    expect(writeText).toHaveBeenCalledWith(created.url);
    expect(await within(dialog).findByRole('list', { name: 'Existing links' })).toBeInTheDocument();
  });

  it('sends password, expiry and view limit when given', async () => {
    const user = userEvent.setup();
    m.sharedLinksService.create.mockResolvedValue({ data: link({ has_password: true }) });
    const dialog = await renderDialog();
    const password = within(dialog).getByLabelText('Password (optional)');
    expect(password).toHaveAttribute('type', 'password');
    await user.type(password, 's3cret');
    await user.click(within(dialog).getByRole('button', { name: 'Show password' }));
    expect(password).toHaveAttribute('type', 'text');
    await user.type(within(dialog).getByLabelText('Expires (optional)'), '2030-01-02T03:04');
    await user.type(within(dialog).getByLabelText('View limit (optional)'), '5');
    await user.click(within(dialog).getByRole('button', { name: 'Create link' }));
    expect(m.sharedLinksService.create).toHaveBeenCalledWith({
      document_id: 'doc-1',
      password: 's3cret',
      expires_at: new Date('2030-01-02T03:04').toISOString(),
      max_views: 5,
    });
    expect(await within(dialog).findByText('People will need the password to open it.')).toBeInTheDocument();
  });

  it('shows the server error when creating fails', async () => {
    const user = userEvent.setup();
    m.sharedLinksService.create.mockRejectedValue(httpError(400, 'Expiry must be in the future'));
    const dialog = await renderDialog();
    await user.click(within(dialog).getByRole('button', { name: 'Create link' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Expiry must be in the future');
  });

  it('lists existing links with status, views and expiry', async () => {
    m.sharedLinksService.listByDocument.mockResolvedValue({
      data: [
        link({ max_views: 10 }),
        link({ id: 'link-2', token: 'zz9', is_expired: true, expires_at: '2025-01-01T00:00:00Z' }),
        link({ id: 'link-3', token: 'yy8', is_revoked: true }),
      ],
    });
    const dialog = await renderDialog();
    const items = within(await within(dialog).findByRole('list', { name: 'Existing links' })).getAllByRole('listitem');
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent('abcdef…wxyz');
    expect(items[0]).toHaveTextContent('Active');
    expect(items[0]).toHaveTextContent('3 / 10');
    expect(items[0]).toHaveTextContent('Never');
    expect(items[1]).toHaveTextContent('Expired');
    expect(within(items[1]).getByRole('button', { name: 'Copy link' })).toBeDisabled();
    expect(items[2]).toHaveTextContent('Revoked');
    expect(within(items[2]).getByRole('button', { name: 'Revoke link' })).toBeDisabled();
  });

  it('revokes a link after confirming', async () => {
    const user = userEvent.setup();
    m.sharedLinksService.listByDocument.mockResolvedValue({ data: [link()] });
    m.sharedLinksService.revoke.mockResolvedValue({});
    const dialog = await renderDialog();
    const item = within(await within(dialog).findByRole('list', { name: 'Existing links' })).getByRole('listitem');
    await user.click(within(item).getByRole('button', { name: 'Revoke link' }));
    const confirm = within(item).getByRole('group', { name: 'Revoke this link?' });
    await user.click(within(confirm).getByRole('button', { name: 'Keep' }));
    expect(m.sharedLinksService.revoke).not.toHaveBeenCalled();

    m.sharedLinksService.listByDocument.mockResolvedValue({ data: [link({ is_revoked: true })] });
    await user.click(within(item).getByRole('button', { name: 'Revoke link' }));
    await user.click(within(item).getByRole('button', { name: 'Revoke' }));
    expect(m.sharedLinksService.revoke).toHaveBeenCalledWith('link-1');
    await waitFor(() => expect(within(dialog).getByRole('listitem')).toHaveTextContent('Revoked'));
  });

  it('reports a failed revoke and keeps the link', async () => {
    const user = userEvent.setup();
    m.sharedLinksService.listByDocument.mockResolvedValue({ data: [link()] });
    m.sharedLinksService.revoke.mockRejectedValue(httpError(500));
    const dialog = await renderDialog();
    const item = within(await within(dialog).findByRole('list', { name: 'Existing links' })).getByRole('listitem');
    await user.click(within(item).getByRole('button', { name: 'Revoke link' }));
    await user.click(within(item).getByRole('button', { name: 'Revoke' }));
    expect(await screen.findByText("Couldn't revoke the link")).toBeInTheDocument();
    expect(item).toHaveTextContent('Active');
  });

  it('shows a retryable error when links cannot load', async () => {
    const user = userEvent.setup();
    m.sharedLinksService.listByDocument.mockRejectedValueOnce(new Error('down'));
    const dialog = await renderDialog();
    expect(await within(dialog).findByRole('alert')).toHaveTextContent("Couldn't load share links.");
    await user.click(within(dialog).getByRole('button', { name: 'Try again' }));
    expect(await within(dialog).findByText('No share links yet.')).toBeInTheDocument();
  });

  it('closes with Done', async () => {
    const user = userEvent.setup();
    const dialog = await renderDialog();
    await user.click(within(dialog).getByRole('button', { name: 'Done' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});
