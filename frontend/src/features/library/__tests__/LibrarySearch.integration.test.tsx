/**
 * Integration tests for Search and the Library together: the whole pages render against mocked
 * services, and a search started in the Library carries on in Search.
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { currentUrl, lastSearchParams, renderLibrary, settle } from './libraryTestUtils';
import { DOCS, hit, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

describe('Search (integration)', () => {
  beforeEach(() => {
    setupLibraryMocks();
    searchService.enhancedSearch.mockResolvedValue(searchResponse([hit(DOCS[0], 'Invoice for March', [[0, 7]])], 1));
  });

  test('renders the Search page with its field and filters', async () => {
    renderLibrary('/search');
    expect(await screen.findByRole('heading', { level: 1, name: 'Search' })).toBeInTheDocument();
    expect(screen.getByRole('searchbox', { name: 'Search documents' })).toBeInTheDocument();
    const strip = screen.getByRole('search', { name: 'Search and filter' });
    for (const name of ['Type', 'Collection', 'Status', 'Source', 'Added']) {
      expect(within(strip).getByRole('button', { name })).toBeInTheDocument();
    }
    await settle();
  });

  test('a search typed in the Library runs on the Search page', async () => {
    const user = userEvent.setup();
    renderLibrary('/documents');
    await user.type(await screen.findByRole('searchbox', { name: 'Search documents' }), 'invoice{Enter}');
    await waitFor(() => expect(currentUrl()).toBe('/search?q=invoice'));
    const link = await screen.findByRole('link', { name: /invoice-march/ });
    expect(link.closest('li')?.querySelector('mark')).toHaveTextContent('Invoice');
    expect(lastSearchParams()).toMatchObject({ query: 'invoice' });
  });
});

describe('Library (performance)', () => {
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
