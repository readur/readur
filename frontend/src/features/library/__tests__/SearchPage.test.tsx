import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useNavigate } from 'react-router-dom';
import { currentUrl, lastSearchParams, lastTimelineParams, renderSearch, settle } from './libraryTestUtils';
import { DOCS, documentService, doc, hit, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

const searchbox = () => screen.getByRole('searchbox', { name: 'Search documents' });
const HITS = [hit(DOCS[0], 'Invoice for March, total due', [[0, 7]]), hit(DOCS[1], 'Lease agreement', [])];
const result = (name: RegExp) => screen.findByRole('link', { name });

describe('Search page', () => {
  beforeEach(() => {
    setupLibraryMocks();
    searchService.enhancedSearch.mockResolvedValue(searchResponse(HITS, 2));
  });

  describe('page', () => {
    test('has one h1 "Advanced search" and a large named field that has focus', async () => {
      renderSearch();
      await settle();
      expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
      expect(screen.getByRole('heading', { level: 1, name: 'Advanced search' })).toBeInTheDocument();
      expect(searchbox()).toHaveFocus();
      expect(searchbox()).toHaveAttribute('placeholder', 'Search every document by name or by the words inside it');
    });

    test('offers the filters: type, collection, source, added and status', async () => {
      renderSearch();
      await settle();
      const strip = screen.getByRole('search', { name: 'Search and filter' });
      for (const name of ['Type', 'Collection', 'Source', 'Added', 'Status']) {
        expect(within(strip).getByRole('button', { name })).toBeInTheDocument();
      }
    });

    test('before a search it explains what can be searched and sends nothing', async () => {
      renderSearch();
      await settle();
      expect(screen.getByRole('heading', { name: 'Find anything in your documents' })).toBeInTheDocument();
      expect(searchService.enhancedSearch).not.toHaveBeenCalled();
      expect(searchService.getTimeline).not.toHaveBeenCalled();
      expect(documentService.listFiltered).not.toHaveBeenCalled();
    });

    test('/search?query= (the old name) becomes q, keeping other parameters', async () => {
      renderSearch('/search?query=tax&status=failed&type=pdf');
      await waitFor(() => expect(currentUrl()).toBe('/search?q=tax&status=failed&type=pdf'));
      await settle();
    });

    test('/documents?q= moves to the Search page with the filters kept', async () => {
      renderSearch('/documents?q=invoice&status=completed');
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice&status=completed'));
      expect(searchbox()).toHaveValue('invoice');
      await result(/invoice-march/);
    });
  });

  describe('search field', () => {
    test('writes the query to the URL after a short pause', async () => {
      const user = userEvent.setup();
      renderSearch();
      await user.type(searchbox(), 'invoice');
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice'));
    });

    test('sends one search for a burst of typing', async () => {
      const user = userEvent.setup();
      renderSearch();
      await user.type(searchbox(), 'invoice');
      await waitFor(() => expect(searchService.enhancedSearch).toHaveBeenCalled());
      expect(searchService.enhancedSearch).toHaveBeenCalledTimes(1);
      expect(lastSearchParams()).toMatchObject({ query: 'invoice' });
    });

    test('Enter searches at once', async () => {
      const user = userEvent.setup();
      renderSearch();
      await user.type(searchbox(), 'lease{Enter}');
      expect(currentUrl()).toBe('/search?q=lease');
    });

    test('the clear button empties the field and ends the search', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await user.click(screen.getByRole('button', { name: 'Clear' }));
      expect(searchbox()).toHaveValue('');
      await waitFor(() => expect(currentUrl()).toBe('/search'));
    });

    test('Escape clears the field', async () => {
      const user = userEvent.setup();
      renderSearch();
      await user.type(searchbox(), 'abc');
      await user.keyboard('{Escape}');
      expect(searchbox()).toHaveValue('');
    });

    test('"/" focuses the field from anywhere else on the page', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      searchbox().blur();
      await user.keyboard('/');
      expect(searchbox()).toHaveFocus();
      expect(searchbox()).toHaveValue('invoice');
    });

    test('"/" claims the key before other window listeners', async () => {
      const user = userEvent.setup();
      const other = vi.fn((e: KeyboardEvent) => e.defaultPrevented);
      window.addEventListener('keydown', other);
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      searchbox().blur();
      await user.keyboard('/');
      expect(other).toHaveReturnedWith(true);
      window.removeEventListener('keydown', other);
    });

    test('a one-character query only hints and sends no search', async () => {
      const user = userEvent.setup();
      renderSearch();
      await user.type(searchbox(), 'x');
      expect((await screen.findAllByText('Keep typing…')).length).toBeGreaterThan(0);
      await waitFor(() => expect(currentUrl()).toBe('/search?q=x'));
      expect(searchService.enhancedSearch).not.toHaveBeenCalled();
    });
  });

  describe('requests', () => {
    test('searches with the filters and snippets, newest first', async () => {
      renderSearch('/search?q=invoice&status=completed&labels=l-tax');
      await result(/invoice-march/);
      expect(lastSearchParams()).toMatchObject({
        query: 'invoice',
        include_snippets: true,
        ocr_status: 'completed',
        label_ids: ['l-tax'],
        limit: 50,
        offset: 0,
        sort_by: 'created_at',
        sort_order: 'desc',
      });
      expect(documentService.listFiltered).not.toHaveBeenCalled();
    });

    test('"Best match" drops the sort so results come back by relevance', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await user.click(screen.getByRole('radio', { name: 'Best match' }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice&sort=relevance'));
      await waitFor(() => expect(lastSearchParams().sort_by).toBeUndefined());
      await user.click(screen.getByRole('radio', { name: 'Newest first' }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice'));
    });

    test('asks for the month histogram with the same filters but every date', async () => {
      renderSearch('/search?q=invoice&status=failed&from=2024-03-01&to=2024-03-31');
      await waitFor(() => expect(searchService.getTimeline).toHaveBeenCalled());
      expect(lastTimelineParams()).toMatchObject({ query: 'invoice', ocr_status: 'failed' });
      expect(lastTimelineParams().created_from).toBeUndefined();
      expect(lastTimelineParams().limit).toBeUndefined();
      expect(lastSearchParams().created_from).toBeInstanceOf(Date);
    });
  });

  describe('results', () => {
    test('shows how many documents match', async () => {
      renderSearch('/search?q=invoice');
      expect(await screen.findByText('2 documents match “invoice”')).toBeInTheDocument();
    });

    test('groups results under month headings', async () => {
      searchService.enhancedSearch.mockResolvedValue(
        searchResponse([
          hit(doc('a', 'new.pdf', { created_at: '2026-09-10T10:00:00Z' }), 'x', []),
          hit(doc('b', 'old.pdf', { created_at: '2024-03-05T10:00:00Z' }), 'x', []),
        ]),
      );
      renderSearch('/search?q=scan');
      expect(await screen.findByRole('heading', { level: 2, name: /September 2026/ })).toBeInTheDocument();
      expect(screen.getByRole('heading', { level: 2, name: /March 2024/ })).toBeInTheDocument();
    });

    test('in relevance order the results are one list without months', async () => {
      renderSearch('/search?q=invoice&sort=relevance');
      await result(/invoice-march/);
      expect(screen.queryByRole('heading', { level: 2 })).not.toBeInTheDocument();
      expect(screen.getByRole('region', { name: 'Results' })).toBeInTheDocument();
    });

    test('shows the matching passage with the words marked', async () => {
      renderSearch('/search?q=invoice');
      const link = await result(/invoice-march/);
      const item = link.closest('li') as HTMLElement;
      expect(within(item).getByText(/total due/)).toBeInTheDocument();
      expect(item.querySelector('mark')).toHaveTextContent('Invoice');
    });

    test('shows the two passages with the most matches, without repeats', async () => {
      const multi = hit(DOCS[0], 'first tax', [[6, 9]]);
      multi.snippets = [
        { text: 'first tax', start_offset: 0, end_offset: 9, highlight_ranges: [{ start: 6, end: 9 }] },
        { text: 'tax and more tax', start_offset: 100, end_offset: 116, highlight_ranges: [{ start: 0, end: 3 }, { start: 13, end: 16 }] },
        { text: 'and more tax again', start_offset: 104, end_offset: 122, highlight_ranges: [{ start: 9, end: 12 }] },
        { text: 'no match here', start_offset: 300, end_offset: 313, highlight_ranges: [] },
      ];
      searchService.enhancedSearch.mockResolvedValue(searchResponse([multi], 1));
      renderSearch('/search?q=tax');
      const item = (await result(/invoice-march/)).closest('li') as HTMLElement;
      const passages = within(item).getAllByRole('listitem').map((li) => li.textContent);
      expect(passages).toEqual(['first tax', 'tax and more tax']);
    });

    test('says when only the file name matched', async () => {
      const bare = hit(DOCS[1], '', []);
      bare.snippets = [];
      searchService.enhancedSearch.mockResolvedValue(searchResponse([bare], 1));
      renderSearch('/search?q=lease');
      await result(/lease\.pdf/);
      expect(screen.getByText('Matched on the file name')).toBeInTheDocument();
    });

    test('names the source and the date of each result', async () => {
      renderSearch('/search?q=invoice');
      const lease = (await result(/lease\.pdf/)).closest('li') as HTMLElement;
      expect(await within(lease).findByText('Office NAS')).toBeInTheDocument();
      const invoice = (await result(/invoice-march/)).closest('li') as HTMLElement;
      expect(within(invoice).getByText('Upload')).toBeInTheDocument();
      expect(invoice.querySelector('time')).toHaveAttribute('dateTime', DOCS[0].created_at);
    });

    test('a result opens the document drawer over the results, keeping the search', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await user.click(await result(/invoice-march/));
      expect(await screen.findByRole('dialog', { name: 'invoice-march.pdf' })).toBeInTheDocument();
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice&document=d1'));
    });

    test('has no separate quick look: the drawer is the document', async () => {
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      expect(screen.queryByRole('button', { name: /quick look/i })).not.toBeInTheDocument();
    });

    test('too many matches asks the user to refine', async () => {
      searchService.enhancedSearch.mockRejectedValue({ response: { status: 413, data: { error_code: 'SEARCH_TOO_MANY_RESULTS' } } });
      renderSearch('/search?q=the');
      expect(await screen.findByRole('heading', { name: 'Too many matches — refine your search' })).toBeInTheDocument();
    });

    test('an error offers to try again', async () => {
      const user = userEvent.setup();
      searchService.enhancedSearch.mockRejectedValueOnce(new Error('down'));
      renderSearch('/search?q=invoice');
      await user.click(await screen.findByRole('button', { name: 'Try again' }));
      await result(/invoice-march/);
    });

    test('no hits suggests checking the spelling', async () => {
      searchService.enhancedSearch.mockResolvedValue(searchResponse([], 0));
      renderSearch('/search?q=zebra');
      expect(await screen.findByRole('heading', { name: 'No matches' })).toBeInTheDocument();
      expect(screen.getByText(/Check the spelling/)).toBeInTheDocument();
    });

    test('no hits with filters offers to clear them', async () => {
      const user = userEvent.setup();
      searchService.enhancedSearch.mockResolvedValue(searchResponse([], 0));
      renderSearch('/search?q=zebra&status=failed');
      await user.click(await screen.findByRole('button', { name: 'Clear filters' }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=zebra'));
    });
  });

  describe('timeline', () => {
    const MONTHS = [
      { month: '2024-01', count: 2 },
      { month: '2024-03', count: 5 },
    ];

    test('draws a bar per month with matches, zero months included as gaps', async () => {
      searchService.getTimeline.mockResolvedValue({ data: MONTHS });
      renderSearch('/search?q=invoice');
      const chart = await screen.findByRole('group', { name: 'Matches by month' });
      const bars = within(chart).getAllByRole('button');
      expect(bars.map((b) => b.getAttribute('aria-label'))).toEqual(['January 2024: 2 matches', 'March 2024: 5 matches']);
    });

    test('pressing a month narrows the dates to it; "Show all dates" widens them again', async () => {
      const user = userEvent.setup();
      searchService.getTimeline.mockResolvedValue({ data: MONTHS });
      renderSearch('/search?q=invoice');
      await user.click(await screen.findByRole('button', { name: 'March 2024: 5 matches' }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice&from=2024-03-01&to=2024-03-31'));
      expect(await screen.findByText(/in March 2024/)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'March 2024: 5 matches' })).toHaveAttribute('aria-pressed', 'true');
      await user.click(screen.getByRole('button', { name: 'Show all dates' }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice'));
    });

    test('shows nothing when the histogram cannot be loaded', async () => {
      searchService.getTimeline.mockRejectedValue(new Error('404'));
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await settle();
      expect(screen.queryByRole('group', { name: 'Matches by month' })).not.toBeInTheDocument();
    });
  });

  describe('selection', () => {
    test('ticking results shows the bar with "Save as collection" first', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await user.click(screen.getByRole('checkbox', { name: 'Select invoice-march.pdf' }));
      const bar = await screen.findByRole('toolbar', { name: 'Bulk actions' });
      expect(within(bar).getAllByRole('button')[0]).toHaveAccessibleName('Save as collection');
    });

    test('select all on the page, then every match', async () => {
      const user = userEvent.setup();
      searchService.enhancedSearch.mockResolvedValue(searchResponse(HITS, 120));
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await user.click(screen.getByRole('checkbox', { name: 'Select all on this page' }));
      expect(screen.getByRole('checkbox', { name: 'Select lease.pdf' })).toBeChecked();
      await user.click(screen.getByRole('button', { name: 'Select all 120 matches' }));
      expect(screen.getByText('All 120 matches are selected')).toBeInTheDocument();
      const bar = screen.getByRole('toolbar', { name: 'Bulk actions' });
      expect(within(bar).getByText('120')).toBeInTheDocument();
    });

    test('a new search clears the selection', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      await user.click(screen.getByRole('checkbox', { name: 'Select invoice-march.pdf' }));
      await user.clear(searchbox());
      await user.type(searchbox(), 'lease{Enter}');
      await waitFor(() => expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument());
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
      renderSearch('/search', <Nav />);
      await user.type(searchbox(), 'lease{Enter}');
      expect(currentUrl()).toBe('/search?q=lease');
      act(() => go(-1));
      await waitFor(() => expect(currentUrl()).toBe('/search'));
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

    test('opens from a named button and explains what is searched', async () => {
      const user = userEvent.setup();
      renderSearch();
      const help = await openHelp(user);
      expect(within(help).getByText(/file names and the text read from each document/)).toBeInTheDocument();
    });

    test('lists the match modes with an explanation each', async () => {
      const user = userEvent.setup();
      renderSearch();
      const help = await openHelp(user);
      const modes = within(help).getByRole('radiogroup', { name: 'Match' });
      expect(within(modes).getAllByRole('radio')).toHaveLength(4);
      expect(within(modes).getByRole('radio', { name: /all words/i })).toBeChecked();
      expect(within(help).getByText('Also finds near misses and typos')).toBeInTheDocument();
    });

    test('changing the mode writes it to the URL and the search uses it', async () => {
      const user = userEvent.setup();
      renderSearch('/search?q=invoice');
      await result(/invoice-march/);
      const help = await openHelp(user);
      await user.click(within(help).getByRole('radio', { name: /similar spelling/i }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice&mode=fuzzy'));
      await waitFor(() => expect(lastSearchParams()).toMatchObject({ search_mode: 'fuzzy' }));
    });

    test('an example runs its search with its mode and closes the help', async () => {
      const user = userEvent.setup();
      renderSearch();
      const help = await openHelp(user);
      const examples = within(help).getByRole('list', { name: 'Try' });
      expect(within(examples).getAllByRole('button').length).toBeGreaterThanOrEqual(5);
      await user.click(within(help).getByRole('button', { name: /invoice & paid/ }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice+%26+paid&mode=boolean'));
      expect(screen.queryByRole('dialog', { name: 'Search help' })).not.toBeInTheDocument();
      expect(searchbox()).toHaveValue('invoice & paid');
      await waitFor(() => expect(lastSearchParams()).toMatchObject({ query: 'invoice & paid', search_mode: 'boolean' }));
    });

    test('a simple example leaves the mode unset', async () => {
      const user = userEvent.setup();
      renderSearch('/search?mode=phrase');
      const help = await openHelp(user);
      await user.click(within(help).getByRole('button', { name: /^invoice documents that mention/i }));
      await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice'));
    });

    test('Escape closes the help and returns focus to its button', async () => {
      const user = userEvent.setup();
      renderSearch();
      await openHelp(user);
      await user.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Search help' })).not.toBeInTheDocument());
      await waitFor(() => expect(screen.getByRole('button', { name: 'Search help' })).toHaveFocus());
    });
  });
});
