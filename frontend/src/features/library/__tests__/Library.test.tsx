import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { currentUrl, lastListParams, renderLibrary } from './libraryTestUtils';
import { DOCS, apiClient, documentService, listResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

const grid = () => screen.getByRole('grid', { name: 'Documents' });
const bodyRows = () => within(grid()).getAllByRole('row').slice(1);
const loaded = () => screen.findByRole('rowheader', { name: /invoice-march\.pdf/ });

describe('Library', () => {
  beforeEach(() => {
    setupLibraryMocks();
  });

  describe('page', () => {
    test('has one h1 "Library" and the total in the header', async () => {
      documentService.listFiltered.mockResolvedValue(listResponse(DOCS, 12408));
      renderLibrary();
      expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
      expect(await screen.findByText('12,408 documents')).toBeInTheDocument();
    });

    test('"Add documents" goes to the upload section', async () => {
      const user = userEvent.setup();
      documentService.listFiltered.mockResolvedValue(listResponse(DOCS, 3));
      renderLibrary();
      await loaded();
      await user.click(screen.getAllByRole('button', { name: 'Add documents' })[0]);
      expect(currentUrl()).toBe('/intake?section=upload');
    });

    test('renders the columns and one row per document', async () => {
      renderLibrary();
      await loaded();
      for (const name of ['Name', 'Type', 'Status', 'Source', 'Labels', 'Size', 'Added']) {
        expect(within(grid()).getByRole('columnheader', { name })).toBeInTheDocument();
      }
      expect(bodyRows()).toHaveLength(3);
    });

    test('shows type, status with progress, source, labels and size in the cells', async () => {
      renderLibrary();
      await loaded();
      const [first, second, third] = bodyRows();
      expect(within(first).getByRole('gridcell', { name: 'PDF' })).toBeInTheDocument();
      expect(within(first).getByText('INDEXED')).toBeInTheDocument();
      expect(within(first).getByText('Upload')).toBeInTheDocument();
      expect(within(first).getByText('Tax')).toBeInTheDocument();
      expect(within(first).getByText('Home')).toBeInTheDocument();
      expect(within(first).getByText('+1')).toBeInTheDocument();
      expect(within(first).getByText('2.0 KB')).toBeInTheDocument();
      expect(await within(second).findByText('Office NAS')).toBeInTheDocument();
      expect(within(second).getByText('FAILED')).toBeInTheDocument();
      expect(within(third).getByText('OCR 3/12')).toBeInTheDocument();
      expect(within(third).getByRole('gridcell', { name: 'PNG' })).toBeInTheDocument();
    });

    test('gives the added date in full next to the relative time', async () => {
      renderLibrary();
      await loaded();
      const first = bodyRows()[0];
      expect(within(first).getByText(/2026/)).toBeInTheDocument();
    });
  });

  describe('list requests', () => {
    test('loads newest first, 50 per page by default', async () => {
      renderLibrary();
      await loaded();
      expect(lastListParams()).toMatchObject({ sort_by: 'created_at', sort_order: 'desc', limit: 50, offset: 0 });
      expect(searchService.enhancedSearch).not.toHaveBeenCalled();
    });

    test.each([
      ['filename', 'asc'],
      ['file_size', 'desc'],
      ['ocr_status', 'asc'],
      ['mime_type', 'desc'],
      ['updated_at', 'asc'],
      ['created_at', 'asc'],
    ])('sends sort=%s order=%s from the URL', async (sort, order) => {
      renderLibrary(`/documents?sort=${sort}&order=${order}`);
      await loaded();
      expect(lastListParams()).toMatchObject({ sort_by: sort, sort_order: order });
    });

    test('ignores an unknown sort in the URL', async () => {
      renderLibrary('/documents?sort=bogus&order=sideways');
      await loaded();
      expect(lastListParams()).toMatchObject({ sort_by: 'created_at', sort_order: 'desc' });
    });

    test('sends each filter from the URL', async () => {
      renderLibrary('/documents?type=pdf,image&labels=l-tax,l-home&status=failed&source=s1&from=2026-01-01&to=2026-02-01&page=3&size=25');
      await loaded();
      const p = lastListParams();
      expect(p).toMatchObject({
        mime_types: ['application/pdf', 'image/png'],
        label_ids: ['l-tax', 'l-home'],
        ocr_status: 'failed',
        source_ids: ['s1'],
        limit: 25,
        offset: 50,
      });
      expect(p.created_from).toBeInstanceOf(Date);
      expect(p.created_to).toBeInstanceOf(Date);
      expect(p.created_from.getDate()).toBe(1);
      expect(p.created_to.getHours()).toBe(23);
    });

    test('filters uploads by source type', async () => {
      renderLibrary('/documents?source=uploaded');
      await loaded();
      expect(lastListParams()).toMatchObject({ source_types: ['direct_upload'] });
      expect(lastListParams().source_ids).toBeUndefined();
    });

    test('maps the Office group to the office types in the library', async () => {
      documentService.getFacets.mockResolvedValue({
        data: {
          mime_types: [
            { value: 'application/pdf', count: 1 },
            { value: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', count: 1 },
          ],
          tags: [],
        },
      });
      renderLibrary('/documents?type=office');
      await loaded();
      expect(lastListParams().mime_types).toEqual(['application/vnd.openxmlformats-officedocument.wordprocessingml.document']);
    });

    test('drops an end date that is before the start date', async () => {
      renderLibrary('/documents?from=2026-03-01&to=2026-01-01');
      await loaded();
      expect(lastListParams().created_from).toBeInstanceOf(Date);
      expect(lastListParams().created_to).toBeUndefined();
    });

    test('fetches the filter values', async () => {
      renderLibrary();
      await loaded();
      expect(documentService.getFacets).toHaveBeenCalled();
      expect(apiClient.get).toHaveBeenCalledWith('/sources');
    });
  });

  describe('URL state', () => {
    test('applies filters from the URL to the chips', async () => {
      renderLibrary('/documents?status=failed&labels=l-tax&source=uploaded&type=pdf');
      await loaded();
      expect(screen.getByRole('button', { name: /status failed/i })).toBeInTheDocument();
      expect(await screen.findByRole('button', { name: /label tax/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /source upload/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /type pdf/i })).toBeInTheDocument();
    });

    test('choosing a status writes it to the URL and refetches', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Status' }));
      await user.click(await screen.findByRole('radio', { name: 'Failed' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?status=failed'));
      await waitFor(() => expect(lastListParams()).toMatchObject({ ocr_status: 'failed' }));
    });

    test('ticking a type writes it to the URL', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Type' }));
      await user.click(await screen.findByRole('checkbox', { name: 'Images' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?type=image'));
      await waitFor(() => expect(lastListParams()).toMatchObject({ mime_types: ['image/png'] }));
    });

    test('the label filter searches and ticks labels', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Label' }));
      await user.type(await screen.findByRole('searchbox', { name: 'Find a label' }), 'ta');
      expect(screen.queryByRole('checkbox', { name: 'Home' })).not.toBeInTheDocument();
      await user.click(screen.getByRole('checkbox', { name: 'Tax' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?labels=l-tax'));
    });

    test('Upload and connections exclude each other in the source filter', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?source=s1');
      await loaded();
      await user.click(await screen.findByRole('button', { name: /^source/i }));
      await user.click(await screen.findByRole('checkbox', { name: 'Upload' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?source=uploaded'));
    });

    test('an Added preset sets the start date', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Added' }));
      await user.click(await screen.findByRole('button', { name: 'Last 30 days' }));
      await waitFor(() => expect(currentUrl()).toMatch(/^\/documents\?from=\d{4}-\d{2}-\d{2}$/));
      expect(screen.getByRole('button', { name: /added last 30 days/i })).toBeInTheDocument();
    });

    test('a chip’s × clears only that filter', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?status=failed&type=pdf');
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Clear Status filter' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?type=pdf'));
    });

    test('"Clear all" removes every filter but keeps the search', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?q=tax&status=failed&type=pdf&labels=l-tax');
      await screen.findByRole('grid');
      await user.click(screen.getByRole('button', { name: 'Clear all' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=tax'));
    });

    test('changing a filter returns to page 1', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?page=4');
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Status' }));
      await user.click(await screen.findByRole('radio', { name: 'Pending' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?status=pending'));
    });

    test('pagination writes the page to the URL', async () => {
      const user = userEvent.setup();
      documentService.listFiltered.mockResolvedValue(listResponse(DOCS, 180));
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: 'Next page' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?page=2'));
      await waitFor(() => expect(lastListParams()).toMatchObject({ offset: 50 }));
    });
  });

  describe('sorting', () => {
    test('a column head writes the sort to the URL', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('columnheader', { name: 'Size' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?sort=file_size&order=asc'));
      await waitFor(() => expect(lastListParams()).toMatchObject({ sort_by: 'file_size', sort_order: 'asc' }));
    });

    test('marks the sorted column', async () => {
      renderLibrary('/documents?sort=filename&order=asc');
      await loaded();
      expect(screen.getByRole('columnheader', { name: 'Name' })).toHaveAttribute('aria-sort', 'ascending');
    });

    test('Source and Labels do not sort', async () => {
      renderLibrary();
      await loaded();
      expect(screen.getByRole('columnheader', { name: 'Source' })).not.toHaveAttribute('aria-sort');
      expect(screen.getByRole('columnheader', { name: 'Labels' })).not.toHaveAttribute('aria-sort');
    });

    test('the narrow-screen sort menu sets field and direction', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await loaded();
      await user.click(screen.getByRole('button', { name: /sort by/i }));
      await user.click(await screen.findByRole('option', { name: 'Name, ascending' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?sort=filename&order=asc'));
    });
  });

  describe('states', () => {
    test('shows skeleton rows while loading', async () => {
      documentService.listFiltered.mockReturnValue(new Promise(() => {}));
      renderLibrary();
      expect(await screen.findByRole('status', { name: 'Loading' })).toBeInTheDocument();
    });

    test('"No documents yet" with an Add action for an empty library', async () => {
      const user = userEvent.setup();
      documentService.listFiltered.mockResolvedValue(listResponse([], 0));
      renderLibrary();
      expect(await screen.findByRole('heading', { name: 'No documents yet' })).toBeInTheDocument();
      const empty = screen.getByRole('heading', { name: 'No documents yet' }).parentElement as HTMLElement;
      await user.click(within(empty).getByRole('button', { name: 'Add documents' }));
      expect(currentUrl()).toBe('/intake?section=upload');
    });

    test('"No matches" with Clear filters when filtered to nothing', async () => {
      const user = userEvent.setup();
      documentService.listFiltered.mockResolvedValue(listResponse([], 0));
      renderLibrary('/documents?status=failed');
      expect(await screen.findByRole('heading', { name: 'No matches' })).toBeInTheDocument();
      await user.click(screen.getByRole('button', { name: 'Clear filters' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents'));
    });

    test('an error offers to try again', async () => {
      const user = userEvent.setup();
      documentService.listFiltered.mockRejectedValueOnce(new Error('boom'));
      renderLibrary();
      expect(await screen.findByRole('heading', { name: 'Documents could not be loaded' })).toBeInTheDocument();
      await user.click(screen.getByRole('button', { name: 'Try again' }));
      expect(await loaded()).toBeInTheDocument();
    });

    test('ignores a slow earlier response that arrives after a newer one', async () => {
      const user = userEvent.setup();
      let resolveSlow!: (value: unknown) => void;
      documentService.listFiltered
        .mockImplementationOnce(() => new Promise((resolve) => (resolveSlow = resolve)))
        .mockResolvedValueOnce(listResponse([DOCS[1]], 1));
      renderLibrary();
      await user.click(screen.getByRole('button', { name: 'Status' }));
      await user.click(await screen.findByRole('radio', { name: 'Failed' }));
      expect(await screen.findByRole('rowheader', { name: /lease\.pdf/ })).toBeInTheDocument();
      await act(async () => resolveSlow(listResponse(DOCS, 3)));
      expect(screen.queryByRole('rowheader', { name: /invoice-march/ })).not.toBeInTheDocument();
      expect(bodyRows()).toHaveLength(1);
      expect(screen.getByText('1 document')).toBeInTheDocument();
    });

    test('hides pagination when there is nothing to page', async () => {
      documentService.listFiltered.mockResolvedValue(listResponse([], 0));
      renderLibrary();
      await screen.findByRole('heading', { name: 'No documents yet' });
      expect(screen.queryByRole('navigation', { name: 'Pagination' })).not.toBeInTheDocument();
    });
  });

  describe('density', () => {
    test('remembers compact rows', async () => {
      const user = userEvent.setup();
      const { unmount } = renderLibrary();
      await loaded();
      await user.click(screen.getByRole('switch', { name: 'Compact rows' }));
      expect(window.localStorage.getItem('readur.library.compact')).toBe('true');
      unmount();
      renderLibrary();
      await loaded();
      expect(screen.getByRole('switch', { name: 'Compact rows' })).toBeChecked();
    });
  });
});
