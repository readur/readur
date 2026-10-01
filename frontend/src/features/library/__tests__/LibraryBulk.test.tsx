import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { currentUrl, renderLibrary, settle } from './libraryTestUtils';
import { documentService, labelService, listResponse, DOCS, retryModal, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

type User = ReturnType<typeof userEvent.setup>;

const rowFor = (name: RegExp) =>
  screen.getByRole('rowheader', { name, hidden: true }).closest('[role="row"]') as HTMLElement;
const bar = () => screen.getByRole('toolbar', { name: 'Bulk actions' });

async function select(user: User, ...names: RegExp[]) {
  await screen.findByRole('rowheader', { name: names[0] });
  for (const name of names) {
    await user.click(within(rowFor(name)).getByRole('checkbox'));
  }
}

describe('Library bulk actions', () => {
  beforeEach(() => {
    setupLibraryMocks();
  });

  test('ticking rows shows the bar with the count', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    expect(within(bar()).getByText('2')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  test('Space on a focused row selects it', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await screen.findByRole('rowheader', { name: /lease\.pdf/ });
    act(() => rowFor(/lease\.pdf/).focus());
    await user.keyboard(' ');
    expect(bar()).toBeInTheDocument();
  });

  test('select all takes every row on the page', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await screen.findByRole('rowheader', { name: /lease\.pdf/ });
    await user.click(screen.getByRole('checkbox', { name: 'Select All' }));
    expect(within(bar()).getByText('3')).toBeInTheDocument();
  });

  test('Clear empties the selection', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: /clear/i }));
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
  });

  test('Add label applies the chosen labels to every selected document', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /photo\.png/);
    await user.click(within(bar()).getByRole('button', { name: 'Add label' }));
    const dialog = await screen.findByRole('dialog', { name: 'Add labels to 2 documents' });
    const apply = within(dialog).getByRole('button', { name: 'Add labels' });
    expect(apply).toBeDisabled();
    await user.click(within(dialog).getByRole('combobox', { name: 'Labels' }));
    await user.click(await screen.findByRole('option', { name: /work/i }));
    await user.click(apply);
    await waitFor(() => expect(labelService.bulkAssign).toHaveBeenCalledWith(['d1', 'd3'], ['l-work'], 'add'));
    expect(await screen.findByText('Labels added to 2 documents')).toBeInTheDocument();
  });

  test('Retry OCR opens the retry dialog for the selection', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Retry OCR' }));
    expect(await screen.findByRole('dialog', { name: 'Retry OCR' })).toHaveTextContent('1 selected for retry');
    expect(retryModal).toHaveBeenLastCalledWith(expect.objectContaining({ open: true, selectedDocumentIds: ['d2'] }));
  });

  test('Download fetches each selected file in turn', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Download' }));
    await waitFor(() => expect(documentService.downloadFile).toHaveBeenCalledTimes(2));
    expect(documentService.downloadFile).toHaveBeenNthCalledWith(1, 'd1', 'invoice-march.pdf');
    expect(documentService.downloadFile).toHaveBeenNthCalledWith(2, 'd2', 'lease.pdf');
  });

  test('Download ignores presses while running and reports the count', async () => {
    const user = userEvent.setup();
    let finishFirst!: () => void;
    documentService.downloadFile
      .mockImplementationOnce(() => new Promise<void>((resolve) => (finishFirst = resolve)))
      .mockResolvedValue(undefined);
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Download' }));
    const busy = within(bar()).getByRole('button', { name: 'Downloading…' });
    expect(busy).toBeDisabled();
    await user.click(busy);
    await act(async () => finishFirst());
    expect(await screen.findByText('2 documents downloaded')).toBeInTheDocument();
    expect(documentService.downloadFile).toHaveBeenCalledTimes(2);
    expect(within(bar()).getByRole('button', { name: 'Download' })).toBeEnabled();
  });

  test('Download reports failures separately', async () => {
    const user = userEvent.setup();
    documentService.downloadFile.mockRejectedValueOnce(new Error('x')).mockResolvedValue(undefined);
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Download' }));
    expect(await screen.findByText('1 download failed')).toBeInTheDocument();
    expect(await screen.findByText('1 document downloaded')).toBeInTheDocument();
  });

  test('Delete asks first; cancelling deletes nothing', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Delete' }));
    const confirm = await screen.findByRole('alertdialog', { name: 'Delete 2 documents?' });
    await user.click(within(confirm).getByRole('button', { name: 'Cancel' }));
    expect(documentService.bulkDelete).not.toHaveBeenCalled();
    expect(bar()).toBeInTheDocument();
  });

  test('confirming Delete removes the selection and reloads', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Delete' }));
    const confirm = await screen.findByRole('alertdialog');
    await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(documentService.bulkDelete).toHaveBeenCalledWith(['d1', 'd2']));
    expect(await screen.findByText('2 documents deleted')).toBeInTheDocument();
    expect(documentService.listFiltered.mock.calls.length).toBeGreaterThan(1);
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
  });

  test('a partial delete says how many were deleted, and only those count as gone', async () => {
    documentService.bulkDelete.mockResolvedValue({
      data: { success: true, deleted_count: 1, failed_count: 1, deleted_documents: ['d1'] },
    });
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Delete' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText('1 of 2 documents deleted')).toBeInTheDocument();
    expect(screen.queryByText('2 documents deleted')).not.toBeInTheDocument();
  });

  test('a delete that removed nothing says so and keeps the selection', async () => {
    documentService.bulkDelete.mockResolvedValue({
      data: { success: false, deleted_count: 0, failed_count: 2, deleted_documents: [] },
    });
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /invoice-march/, /lease\.pdf/);
    await user.click(within(bar()).getByRole('button', { name: 'Delete' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText('No documents were deleted')).toBeInTheDocument();
    expect(screen.queryByText(/documents deleted/)).not.toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(within(bar()).getByText('2')).toBeInTheDocument();
  });

  test('changing a filter clears the selection', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /lease\.pdf/);
    await user.click(screen.getByRole('button', { name: 'Status' }));
    await user.click(await screen.findByRole('radio', { name: 'Failed' }));
    await waitFor(() => expect(currentUrl()).toBe('/documents?status=failed'));
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
    await settle();
  });

  test('changing the page clears the selection', async () => {
    const user = userEvent.setup();
    documentService.listFiltered.mockResolvedValue(listResponse(DOCS, 120));
    renderLibrary();
    await select(user, /lease\.pdf/);
    await user.click(screen.getByRole('button', { name: 'Next page' }));
    await waitFor(() => expect(currentUrl()).toBe('/documents?page=2'));
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
    await settle();
  });

  test('searching leaves for the Search page and drops the selection', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await select(user, /lease\.pdf/);
    await user.type(screen.getByRole('searchbox', { name: 'Search documents' }), 'tax{Enter}');
    await waitFor(() => expect(currentUrl()).toBe('/search?q=tax'));
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
    await settle();
  });
});
