import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { nameFromQuery } from '../search/SaveCollectionDialog';
import { renderSearch } from './libraryTestUtils';
import { DOCS, LABELS, hit, label, labelService, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

const HITS = [hit(DOCS[0], 'shoulder pain', [[0, 8]]), hit(DOCS[1], 'shoulder scan', [[0, 8]])];

async function openDialog(user: ReturnType<typeof userEvent.setup>, url = '/search?q=shoulder') {
  renderSearch(url);
  await screen.findByRole('link', { name: /invoice-march/ });
  await user.click(screen.getByRole('checkbox', { name: 'Select all on this page' }));
  await user.click(within(await screen.findByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Save as collection' }));
  return screen.findByRole('dialog', { name: 'Save 2 documents as a collection' });
}

describe('Save as collection', () => {
  const changed = vi.fn();
  beforeEach(() => {
    setupLibraryMocks();
    searchService.enhancedSearch.mockResolvedValue(searchResponse(HITS, 2));
    window.addEventListener('readur:labels-changed', changed);
  });
  afterEach(() => {
    window.removeEventListener('readur:labels-changed', changed);
    changed.mockReset();
  });

  test('suggests the search as the name and a colour no collection uses yet', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    expect(within(dialog).getByRole('textbox', { name: /name/i })).toHaveValue('Shoulder');
    // The mock labels all use the first preset (blue), so the next one is offered.
    expect(within(dialog).getByRole('radio', { name: 'Red' })).toBeChecked();
  });

  test('creates the collection, adds the documents and tells the sidebar', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    const name = within(dialog).getByRole('textbox', { name: /name/i });
    await user.clear(name);
    await user.type(name, 'Shoulder injury');
    await user.click(within(dialog).getByRole('radio', { name: 'Green' }));
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(labelService.create).toHaveBeenCalledWith({ name: 'Shoulder injury', color: '#28a745' }));
    expect(labelService.bulkAssign).toHaveBeenCalledWith(['d1', 'd2'], ['l-new'], 'add');
    expect(changed).toHaveBeenCalled();
    expect(await screen.findByText('2 documents saved to “Shoulder injury”')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
  });

  test('Enter in the name field saves', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    await user.type(within(dialog).getByRole('textbox', { name: /name/i }), ' notes{Enter}');
    await waitFor(() => expect(labelService.create).toHaveBeenCalledWith(expect.objectContaining({ name: 'Shoulder notes' })));
  });

  test('a name in use points to the existing collection instead', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    const name = within(dialog).getByRole('textbox', { name: /name/i });
    await user.clear(name);
    await user.type(name, 'tax');
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(await within(dialog).findByText(/A collection with this name exists/)).toBeInTheDocument();
    expect(labelService.create).not.toHaveBeenCalled();
  });

  test('a comma in the name is refused', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    await user.type(within(dialog).getByRole('textbox', { name: /name/i }), ', knee');
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(await within(dialog).findByText('Label names cannot contain commas.')).toBeInTheDocument();
  });

  test('an empty name cannot be saved', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    await user.clear(within(dialog).getByRole('textbox', { name: /name/i }));
    expect(within(dialog).getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  test('adds the documents to an existing collection', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    await user.click(within(dialog).getByRole('radio', { name: 'An existing collection' }));
    await user.click(within(dialog).getByRole('button', { name: /collection/i }));
    await user.click(await screen.findByRole('option', { name: 'Home' }));
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(labelService.bulkAssign).toHaveBeenCalledWith(['d1', 'd2'], [LABELS[1].id], 'add'));
    expect(labelService.create).not.toHaveBeenCalled();
    expect(await screen.findByText('2 documents saved to “Home”')).toBeInTheDocument();
  });

  test('a refused name shows the server’s reason under the field', async () => {
    const user = userEvent.setup();
    labelService.create.mockRejectedValue(Object.assign(new Error('x'), { isAxiosError: true, response: { status: 400, data: { error: 'Name too long' } } }));
    const dialog = await openDialog(user);
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(await within(dialog).findByText('Name too long')).toBeInTheDocument();
  });

  test('if adding fails after the collection was made, retrying uses that collection', async () => {
    const user = userEvent.setup();
    labelService.bulkAssign.mockRejectedValueOnce(new Error('down'));
    const dialog = await openDialog(user);
    labelService.list.mockResolvedValue({ data: [...LABELS, label('l-new', 'Shoulder')] });
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    expect(await screen.findByText(/The collection was made, but the documents could not be added/)).toBeInTheDocument();
    expect(within(dialog).getByRole('radio', { name: 'An existing collection' })).toBeChecked();
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(labelService.bulkAssign).toHaveBeenLastCalledWith(['d1', 'd2'], ['l-new'], 'add'));
    expect(labelService.create).toHaveBeenCalledTimes(1);
  });

  test('every match: fetches all the ids and saves them', async () => {
    const user = userEvent.setup();
    searchService.enhancedSearch.mockImplementation(async (params: { include_snippets?: boolean; offset?: number }) =>
      params.include_snippets === false
        ? searchResponse(params.offset === 0 ? [hit(DOCS[0], '', []), hit(DOCS[1], '', []), hit(DOCS[2], '', [])] : [], 3)
        : searchResponse(HITS, 3),
    );
    renderSearch('/search?q=shoulder');
    await screen.findByRole('link', { name: /invoice-march/ });
    await user.click(screen.getByRole('checkbox', { name: 'Select all on this page' }));
    await user.click(screen.getByRole('button', { name: 'Select all 3 matches' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Save as collection' }));
    const dialog = await screen.findByRole('dialog', { name: 'Save 3 documents as a collection' });
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(labelService.bulkAssign).toHaveBeenCalledWith(['d1', 'd2', 'd3'], ['l-new'], 'add'));
    const idCall = searchService.enhancedSearch.mock.calls.find(([p]) => p.include_snippets === false)?.[0];
    expect(idCall).toMatchObject({ query: 'shoulder', limit: 1000, offset: 0 });
  });

  test('Cancel closes without saving', async () => {
    const user = userEvent.setup();
    const dialog = await openDialog(user);
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(labelService.create).not.toHaveBeenCalled();
  });
});

describe('nameFromQuery', () => {
  test('drops operators and capitalises', () => {
    expect(nameFromQuery('shoulder & injury')).toBe('Shoulder injury');
    expect(nameFromQuery('"knee pain"')).toBe('Knee pain');
    expect(nameFromQuery('  ')).toBe('');
  });
});
