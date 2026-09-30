/**
 * Integration tests for search in the Library (formerly SearchPage.integration.test.tsx). The
 * whole Library renders against mocked services, reached through the /search redirect.
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { currentUrl, lastSearchParams, renderLibrary, settle } from './libraryTestUtils';
import { DOCS, hit, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

describe('Search in the Library (integration)', () => {
  beforeEach(() => {
    setupLibraryMocks();
    searchService.enhancedSearch.mockResolvedValue(searchResponse([hit(DOCS[0], 'Invoice for March', [[0, 7]])], 1));
  });

  test('renders without crashing', async () => {
    renderLibrary('/search');
    expect(await screen.findByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    await settle();
  });

  test('contains the search field', async () => {
    renderLibrary('/search');
    expect(await screen.findByRole('searchbox', { name: 'Search documents' })).toBeInTheDocument();
    await settle();
  });

  test('shows the basic interface: heading, search, filters and the board', async () => {
    renderLibrary('/search');
    expect(await screen.findByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    const strip = screen.getByRole('search', { name: 'Search and filter' });
    for (const name of ['Type', 'Label', 'Status', 'Source', 'Added']) {
      expect(within(strip).getByRole('button', { name })).toBeInTheDocument();
    }
    expect(await screen.findByRole('grid', { name: 'Documents' })).toBeInTheDocument();
    await settle();
  });

  test('/search?q= lands on the Library searching for q', async () => {
    renderLibrary('/search?q=invoice');
    await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice'));
    expect(screen.getByRole('searchbox', { name: 'Search documents' })).toHaveValue('invoice');
    await screen.findByRole('rowheader', { name: /invoice-march/ });
    expect(lastSearchParams()).toMatchObject({ query: 'invoice' });
  });

  test('/search?query= (the old name) also works, keeping other parameters', async () => {
    renderLibrary('/search?query=tax&status=failed&type=pdf');
    await waitFor(() => expect(currentUrl()).toBe('/documents?q=tax&status=failed&type=pdf'));
    await settle();
  });

  test('a search typed on the page runs end to end', async () => {
    const user = userEvent.setup();
    renderLibrary('/search');
    await user.type(await screen.findByRole('searchbox', { name: 'Search documents' }), 'invoice{Enter}');
    await waitFor(() => expect(currentUrl()).toBe('/documents?q=invoice'));
    const row = (await screen.findByRole('rowheader', { name: /invoice-march/ })).closest('[role="row"]') as HTMLElement;
    expect(row.querySelector('mark')).toHaveTextContent('Invoice');
  });
});

describe('Search in the Library (performance)', () => {
  beforeEach(() => {
    setupLibraryMocks();
  });

  test('renders quickly', async () => {
    const start = performance.now();
    renderLibrary('/documents');
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(1000);
    expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    await settle();
  });
});
