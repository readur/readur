import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { markLit } from '../../board/litStore';
import { currentUrl, lastListParams, renderLibrary, settle, useGrid } from './libraryTestUtils';
import { DOCS, LABELS, doc, documentService, labelService, label, listResponse, setupLibraryMocks } from './serviceMocks';
import { LIBRARY_OCR_POLL_MS } from '../useLibraryData';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

const card = (name: RegExp) => screen.findByRole('button', { name });
const layout = () => screen.getByRole('radiogroup', { name: 'Layout' });

describe('Library grid', () => {
  beforeEach(() => {
    setupLibraryMocks();
    useGrid();
  });

  test('is the default layout and shows a card per document', async () => {
    window.localStorage.removeItem('readur.library.view');
    renderLibrary();
    await card(/invoice-march\.pdf/);
    expect(screen.queryByRole('grid', { name: 'Documents' })).not.toBeInTheDocument();
    expect(within(layout()).getByRole('radio', { name: 'Grid' })).toBeChecked();
    for (const d of DOCS) expect(screen.getByRole('checkbox', { name: `Select ${d.original_filename}` })).toBeInTheDocument();
  });

  test('switching to the table is remembered', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await card(/invoice-march\.pdf/);
    await user.click(within(layout()).getByRole('radio', { name: 'Table' }));
    expect(await screen.findByRole('grid', { name: 'Documents' })).toBeInTheDocument();
    expect(window.localStorage.getItem('readur.library.view')).toBe('table');
  });

  test('groups cards under month headings when newest first', async () => {
    documentService.listFiltered.mockResolvedValue(
      listResponse([doc('a', 'new.pdf', { created_at: '2026-09-10T10:00:00Z' }), doc('b', 'old.pdf', { created_at: '2025-01-05T10:00:00Z' })]),
    );
    renderLibrary();
    expect(await screen.findByRole('heading', { level: 2, name: /September 2026/ })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: /January 2025/ })).toHaveTextContent('1 document');
  });

  test('sorted by name the cards are one grid without months', async () => {
    renderLibrary('/documents?sort=filename&order=asc');
    await card(/invoice-march\.pdf/);
    expect(screen.queryByRole('heading', { level: 2 })).not.toBeInTheDocument();
    expect(lastListParams()).toMatchObject({ sort_by: 'filename', sort_order: 'asc' });
  });

  test('the sort menu sets field and direction', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await card(/invoice-march\.pdf/);
    await user.click(screen.getByRole('button', { name: /sort by/i }));
    await user.click(await screen.findByRole('option', { name: 'Size, descending' }));
    await waitFor(() => expect(currentUrl()).toBe('/documents?sort=file_size&order=desc'));
  });

  test('a card names its source and always shows its OCR status', async () => {
    renderLibrary();
    const lease = (await card(/lease\.pdf/)).closest('li') as HTMLElement;
    expect(await within(lease).findByText('Office NAS')).toBeInTheDocument();
    expect(within(lease).getByText(/failed/i)).toBeInTheDocument();
    const invoice = (await card(/invoice-march\.pdf/)).closest('li') as HTMLElement;
    expect(within(invoice).getByText('Upload')).toBeInTheDocument();
    expect(within(invoice).getByText('Indexed')).toBeInTheDocument();
    const photo = (await card(/photo\.png/)).closest('li') as HTMLElement;
    expect(within(photo).getByText('OCR 3/12')).toBeInTheDocument();
    expect(photo.querySelector('[data-lead] svg')).not.toBeNull(); // the in-progress spinner
  });

  test('a card shows type and size, its first labels, and the source icon', async () => {
    renderLibrary();
    const invoice = (await card(/invoice-march\.pdf/)).closest('li') as HTMLElement;
    expect(within(invoice).getByText(/^PDF · 2\.0 KB · /)).toBeInTheDocument();
    expect(within(invoice).getByText('Tax')).toBeInTheDocument();
    expect(within(invoice).getByText('Home')).toBeInTheDocument();
    expect(within(invoice).queryByText('Work')).not.toBeInTheDocument();
    expect(within(invoice).getByLabelText('1 more')).toHaveTextContent('+1');
    expect(invoice.querySelector('[data-tile] svg')).not.toBeNull();
  });

  test('a new document carries the New marker until opened', async () => {
    const user = userEvent.setup();
    markLit('document', 'd1', 'new');
    renderLibrary();
    const invoice = (await card(/invoice-march\.pdf/)).closest('li') as HTMLElement;
    expect(within(invoice).getByText('New')).toBeInTheDocument();
    await user.click(await card(/invoice-march\.pdf/));
    await waitFor(() => expect(within(invoice).queryByText('New')).not.toBeInTheDocument());
  });

  test('clicking a card opens the detail panel', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await user.click(await card(/lease\.pdf/));
    expect(await screen.findByRole('dialog', { name: /lease\.pdf/ })).toBeInTheDocument();
  });

  test('clicking the preview in the detail panel opens the document', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await user.click(await card(/lease\.pdf/));
    const panel = await screen.findByRole('dialog', { name: /lease\.pdf/ });
    await user.click(within(panel).getByRole('link', { name: 'Open lease.pdf' }));
    await waitFor(() => expect(currentUrl()).toBe('/documents/d2'));
  });

  test('ticking cards selects them for the bulk bar', async () => {
    const user = userEvent.setup();
    renderLibrary();
    await card(/invoice-march\.pdf/);
    await user.click(screen.getByRole('checkbox', { name: 'Select lease.pdf' }));
    await user.click(screen.getByRole('checkbox', { name: 'Select photo.png' }));
    const bar = await screen.findByRole('toolbar', { name: 'Bulk actions' });
    expect(within(bar).getByText('2')).toBeInTheDocument();
    await user.click(screen.getByRole('checkbox', { name: 'Select photo.png' }));
    expect(within(bar).getByText('1')).toBeInTheDocument();
  });

  test('the empty library offers to add documents', async () => {
    documentService.listFiltered.mockResolvedValue(listResponse([], 0));
    renderLibrary();
    expect(await screen.findByRole('heading', { name: 'No documents yet' })).toBeInTheDocument();
  });

  test('shows placeholder cards while loading', async () => {
    documentService.listFiltered.mockReturnValue(new Promise(() => undefined));
    renderLibrary();
    expect(await screen.findByRole('status', { name: 'Loading documents' })).toBeInTheDocument();
  });
});

describe('Collection page', () => {
  beforeEach(() => {
    setupLibraryMocks();
    useGrid();
  });

  test('?label= shows the collection name as the page title and filters by it', async () => {
    renderLibrary('/documents?label=l-tax');
    expect(await screen.findByRole('heading', { level: 1, name: 'Tax' })).toBeInTheDocument();
    await waitFor(() => expect(lastListParams()).toMatchObject({ label_ids: ['l-tax'] }));
  });

  test('shows the collection description under the title', async () => {
    labelService.list.mockResolvedValue({ data: [label('l-med', 'Shoulder injury', { description: 'Physio and scans' })] });
    renderLibrary('/documents?label=l-med');
    expect(await screen.findByText('Physio and scans')).toBeInTheDocument();
  });

  test('an empty collection says how to fill it', async () => {
    const user = userEvent.setup();
    documentService.listFiltered.mockResolvedValue(listResponse([], 0));
    renderLibrary('/documents?label=l-home');
    expect(await screen.findByRole('heading', { name: 'Nothing in “Home” yet' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Go to search' }));
    expect(currentUrl()).toBe('/search');
  });

  test('two collections at once are a plain filtered Library', async () => {
    renderLibrary(`/documents?labels=${LABELS[0].id},${LABELS[1].id}`);
    expect(await screen.findByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    await settle();
  });

  test('picks up labels made elsewhere', async () => {
    renderLibrary('/documents?label=l-new');
    await settle();
    expect(screen.getByRole('heading', { level: 1, name: 'Library' })).toBeInTheDocument();
    labelService.list.mockResolvedValue({ data: [...LABELS, label('l-new', 'Knee')] });
    window.dispatchEvent(new CustomEvent('readur:labels-changed'));
    expect(await screen.findByRole('heading', { level: 1, name: 'Knee' })).toBeInTheDocument();
  });
});

describe('OCR progress', () => {
  beforeEach(() => {
    setupLibraryMocks();
    useGrid();
  });

  test('refreshes quietly while a document is still in OCR, and stops once none is', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      renderLibrary();
      await card(/photo\.png/);
      const calls = documentService.listFiltered.mock.calls.length;
      documentService.listFiltered.mockResolvedValue(
        listResponse([doc('d3', 'photo.png', { mime_type: 'image/png', ocr_status: 'completed' })]),
      );
      await vi.advanceTimersByTimeAsync(LIBRARY_OCR_POLL_MS);
      await waitFor(() => expect(documentService.listFiltered.mock.calls.length).toBe(calls + 1));
      expect(screen.queryByRole('status', { name: 'Loading documents' })).not.toBeInTheDocument();
      expect(await screen.findByText('Indexed')).toBeInTheDocument();
      await vi.advanceTimersByTimeAsync(LIBRARY_OCR_POLL_MS * 2);
      expect(documentService.listFiltered.mock.calls.length).toBe(calls + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
