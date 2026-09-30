import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useNavigate } from 'react-router-dom';
import { currentUrl, lastSearchParams, renderLibrary, settle } from './libraryTestUtils';
import { DOCS, documentService, hit, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

const searchbox = () => screen.getByRole('searchbox', { name: 'Search documents' });
const HITS = [hit(DOCS[0], 'Invoice for March, total due', [[0, 7]]), hit(DOCS[1], 'Lease agreement', [])];

describe('Library search', () => {
  beforeEach(() => {
    setupLibraryMocks();
    searchService.enhancedSearch.mockResolvedValue(searchResponse(HITS, 2));
  });

  describe('search field', () => {
    test('renders a named search field with a placeholder', async () => {
      renderLibrary();
      await settle();
      expect(searchbox()).toHaveAttribute('placeholder', 'Search names and text…');
    });

    test('accepts typing', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'test query');
      expect(searchbox()).toHaveValue('test query');
    });

    test('writes the query to the URL after a short pause', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'invoice');
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice'));
    });

    test('sends one search for a burst of typing', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'invoice');
      await waitFor(() => expect(searchService.enhancedSearch).toHaveBeenCalled());
      expect(searchService.enhancedSearch).toHaveBeenCalledTimes(1);
      expect(lastSearchParams()).toMatchObject({ query: 'invoice' });
    });

    test('Enter searches at once', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'lease{Enter}');
      expect(currentUrl()).toBe('/documents?q=lease');
    });

    test('the clear button empties the field and leaves search mode', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?q=invoice');
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      await user.click(screen.getByRole('button', { name: 'Clear' }));
      expect(searchbox()).toHaveValue('');
      await waitFor(() => expect(currentUrl()).toBe('/documents'));
    });

    test('Escape clears the field', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'abc');
      await user.keyboard('{Escape}');
      expect(searchbox()).toHaveValue('');
    });

    test('an empty field sends no search', async () => {
      renderLibrary();
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      expect(searchService.enhancedSearch).not.toHaveBeenCalled();
    });

    test('"/" focuses the field from anywhere else on the page', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      await user.keyboard('/');
      expect(searchbox()).toHaveFocus();
      expect(searchbox()).toHaveValue('');
    });

    test('"/" claims the key before other window listeners', async () => {
      const user = userEvent.setup();
      const other = vi.fn((e: KeyboardEvent) => e.defaultPrevented);
      window.addEventListener('keydown', other);
      renderLibrary();
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      await user.keyboard('/');
      expect(other).toHaveReturnedWith(true);
      window.removeEventListener('keydown', other);
    });

    test('shows the query from the URL', async () => {
      renderLibrary('/documents?q=tax');
      await settle();
      expect(searchbox()).toHaveValue('tax');
    });
  });

  describe('search mode', () => {
    test('switches to the search endpoint with the filters and snippets', async () => {
      renderLibrary('/documents?q=invoice&status=completed&labels=l-tax');
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      expect(lastSearchParams()).toMatchObject({
        query: 'invoice',
        include_snippets: true,
        ocr_status: 'completed',
        label_ids: ['l-tax'],
        limit: 50,
        offset: 0,
      });
      expect(documentService.listFiltered).not.toHaveBeenCalled();
    });

    test('orders by relevance unless a sort is chosen', async () => {
      renderLibrary('/documents?q=invoice');
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      expect(lastSearchParams().sort_by).toBeUndefined();
      expect(screen.getByRole('columnheader', { name: 'Added' })).not.toHaveAttribute('aria-sort', 'descending');
    });

    test('passes an explicit sort through', async () => {
      renderLibrary('/documents?q=invoice&sort=filename&order=asc');
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      expect(lastSearchParams()).toMatchObject({ sort_by: 'filename', sort_order: 'asc' });
    });

    test('shows the best snippet under the name with the match marked', async () => {
      renderLibrary('/documents?q=invoice');
      await settle();
      const row = (await screen.findByRole('rowheader', { name: /invoice-march/ })).closest('[role="row"]') as HTMLElement;
      expect(row).toHaveAccessibleDescription('Invoice for March, total due');
      const mark = row.querySelector('mark');
      expect(mark).toHaveTextContent('Invoice');
    });

    test('picks the snippet with the most matches', async () => {
      const multi = hit(DOCS[0], 'first', [[0, 5]]);
      multi.snippets.push({ text: 'tax and more tax', start_offset: 0, end_offset: 16, highlight_ranges: [{ start: 0, end: 3 }, { start: 13, end: 16 }] });
      searchService.enhancedSearch.mockResolvedValue(searchResponse([multi], 1));
      renderLibrary('/documents?q=tax');
      await settle();
      const row = (await screen.findByRole('rowheader', { name: /invoice-march/ })).closest('[role="row"]') as HTMLElement;
      expect(row).toHaveAccessibleDescription('tax and more tax');
      expect(row.querySelectorAll('mark')).toHaveLength(2);
    });

    test('a one-character query only hints and sends no search', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await user.type(searchbox(), 'x');
      expect(await screen.findByText('Keep typing…')).toBeInTheDocument();
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=x'));
      expect(searchService.enhancedSearch).not.toHaveBeenCalled();
    });

    test('too many matches asks the user to refine', async () => {
      searchService.enhancedSearch.mockRejectedValue({ response: { status: 413, data: { error_code: 'SEARCH_TOO_MANY_RESULTS' } } });
      renderLibrary('/documents?q=the');
      await settle();
      expect(await screen.findByRole('heading', { name: 'Too many matches — refine your search' })).toBeInTheDocument();
    });

    test('no hits shows "No matches"', async () => {
      searchService.enhancedSearch.mockResolvedValue(searchResponse([], 0));
      renderLibrary('/documents?q=zebra');
      await settle();
      expect(await screen.findByRole('heading', { name: 'No matches' })).toBeInTheDocument();
    });

    test('shows the number of matches', async () => {
      renderLibrary('/documents?q=invoice');
      await settle();
      expect(await screen.findByText('2 documents')).toBeInTheDocument();
    });

    test('the sort menu offers "Best match" while searching', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?q=invoice&sort=filename&order=asc');
      await settle();
      await screen.findByRole('rowheader', { name: /invoice-march/ });
      await user.click(screen.getByRole('button', { name: /sort by/i }));
      await user.click(await screen.findByRole('option', { name: 'Best match' }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice'));
    });
  });

  describe('history', () => {
    test('Back and Forward move between searches', async () => {
      let go: ReturnType<typeof useNavigate> = () => undefined;
      function Nav() {
        go = useNavigate();
        return null;
      }
      const user = userEvent.setup();
      renderLibrary('/documents', <Nav />);
      await settle();
      await user.type(searchbox(), 'lease{Enter}');
      expect(currentUrl()).toBe('/documents?q=lease');
      act(() => go(-1));
      await waitFor(() => expect(currentUrl()).toBe('/documents'));
      expect(searchbox()).toHaveValue('');
      act(() => go(1));
      await waitFor(() => expect(searchbox()).toHaveValue('lease'));
    });
  });

  describe('search help', () => {
    const openHelp = async (user: ReturnType<typeof userEvent.setup>) => {
      await user.click(screen.getByRole('button', { name: 'Search help' }));
      return screen.findByRole('dialog', { name: 'Search help' });
    };

    test('opens from a named button', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      const help = await openHelp(user);
      expect(within(help).getByText(/file names and the text read from each document/)).toBeInTheDocument();
    });

    test('lists the match modes with an explanation each', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      const help = await openHelp(user);
      const modes = within(help).getByRole('radiogroup', { name: 'Match' });
      expect(within(modes).getAllByRole('radio')).toHaveLength(4);
      expect(within(modes).getByRole('radio', { name: /all words/i })).toBeChecked();
      expect(within(help).getByText('Also finds near misses and typos')).toBeInTheDocument();
    });

    test('changing the mode writes it to the URL and the search uses it', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?q=invoice');
      await settle();
      const help = await openHelp(user);
      await user.click(within(help).getByRole('radio', { name: /similar spelling/i }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice&mode=fuzzy'));
      await waitFor(() => expect(lastSearchParams()).toMatchObject({ search_mode: 'fuzzy' }));
    });

    test('shows example searches', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      const help = await openHelp(user);
      const examples = within(help).getByRole('list', { name: 'Try' });
      expect(within(examples).getAllByRole('button').length).toBeGreaterThanOrEqual(5);
      expect(within(examples).getByRole('button', { name: /budget \| forecast/ })).toBeInTheDocument();
    });

    test('an example runs its search with its mode and closes the help', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      const help = await openHelp(user);
      await user.click(within(help).getByRole('button', { name: /invoice & paid/ }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice+%26+paid&mode=boolean'));
      expect(screen.queryByRole('dialog', { name: 'Search help' })).not.toBeInTheDocument();
      expect(searchbox()).toHaveValue('invoice & paid');
      await waitFor(() => expect(lastSearchParams()).toMatchObject({ query: 'invoice & paid', search_mode: 'boolean' }));
    });

    test('a simple example leaves the mode unset', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?mode=phrase');
      await settle();
      const help = await openHelp(user);
      await user.click(within(help).getByRole('button', { name: /^invoice documents that mention/i }));
      await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice'));
    });

    test('Escape closes the help and returns focus to its button', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await settle();
      await openHelp(user);
      await user.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Search help' })).not.toBeInTheDocument());
      await waitFor(() => expect(screen.getByRole('button', { name: 'Search help' })).toHaveFocus());
    });
  });
});
