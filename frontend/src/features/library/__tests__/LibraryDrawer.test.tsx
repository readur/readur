import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useNavigate } from 'react-router-dom';
import { isLit, markLit } from '../../board/litStore';
import { currentUrl, renderLibrary, settle } from './libraryTestUtils';
import {
  DOCS,
  documentService,
  hit,
  labelService,
  listResponse,
  searchResponse,
  searchService,
  setupLibraryMocks,
} from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

type User = ReturnType<typeof userEvent.setup>;

// `hidden`: while the drawer is open, the page behind it is hidden from assistive tech.
const rowFor = (name: RegExp) =>
  screen.getByRole('rowheader', { name, hidden: true }).closest('[role="row"]') as HTMLElement;
const drawerNamed = (name: string | RegExp) => screen.findByRole('dialog', { name });

async function openWithEnter(user: User, name: RegExp) {
  await screen.findByRole('rowheader', { name });
  const row = rowFor(name);
  act(() => row.focus());
  await user.keyboard('{Enter}');
  return screen.findByRole('dialog');
}

function BackButton() {
  const navigate = useNavigate();
  return (
    <button type="button" onClick={() => navigate(-1)}>
      browser back
    </button>
  );
}

describe('Library and the document drawer', () => {
  beforeEach(() => {
    setupLibraryMocks();
  });

  describe('opening and moving', () => {
    test('Enter on a row opens the drawer and puts the document in the URL', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents?labels=l-tax');
      await openWithEnter(user, /invoice-march/);
      expect(await drawerNamed('invoice-march.pdf')).toBeInTheDocument();
      expect(currentUrl()).toBe('/documents?labels=l-tax&document=d1');
      await settle();
    });

    test('clicking a row opens the drawer without selecting it', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await user.click(await screen.findByRole('rowheader', { name: /lease\.pdf/ }));
      expect(await drawerNamed('lease.pdf')).toBeInTheDocument();
      expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
      await settle();
    });

    test('opening a document does not fetch the list again', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await screen.findByRole('rowheader', { name: /lease\.pdf/ });
      const calls = documentService.listFiltered.mock.calls.length;
      await openWithEnter(user, /lease\.pdf/);
      await drawerNamed('lease.pdf');
      expect(documentService.listFiltered.mock.calls.length).toBe(calls);
      await settle();
    });

    test('↓ and ↑ walk the rows of the page and stop at its ends', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await drawerNamed('invoice-march.pdf');
      await user.keyboard('{ArrowUp}');
      expect(currentUrl()).toBe('/documents?document=d1');
      await user.keyboard('{ArrowDown}');
      expect(await drawerNamed('lease.pdf')).toBeInTheDocument();
      await user.keyboard('{ArrowDown}');
      expect(await drawerNamed('photo.png')).toBeInTheDocument();
      await user.keyboard('{ArrowDown}');
      expect(currentUrl()).toBe('/documents?document=d3');
      await user.keyboard('{ArrowUp}');
      expect(await drawerNamed('lease.pdf')).toBeInTheDocument();
      await settle();
    });

    test('Escape closes the drawer, returns focus to the row and drops ?document=', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await drawerNamed('lease.pdf');
      await user.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      await waitFor(() => expect(rowFor(/lease\.pdf/)).toHaveFocus());
      expect(currentUrl()).toBe('/documents');
    });

    test('the browser Back button closes the drawer', async () => {
      const user = userEvent.setup();
      renderLibrary('/documents', <BackButton />);
      await openWithEnter(user, /lease\.pdf/);
      await drawerNamed('lease.pdf');
      fireEvent.click(screen.getByRole('button', { name: 'browser back', hidden: true }));
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      expect(currentUrl()).toBe('/documents');
    });

    test('a link to /documents?document= opens the drawer over the Library', async () => {
      renderLibrary('/documents?document=d2');
      expect(await drawerNamed('lease.pdf')).toBeInTheDocument();
      expect(await screen.findByRole('rowheader', { name: /lease\.pdf/, hidden: true })).toBeInTheDocument();
      await settle();
    });
  });

  describe('changes made in the drawer', () => {
    test('removing a label updates the row without reloading the list', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      const drawer = await drawerNamed('invoice-march.pdf');
      const calls = documentService.listFiltered.mock.calls.length;
      await user.click(await within(drawer).findByRole('button', { name: 'Remove Tax' }));
      await waitFor(() => expect(labelService.setDocumentLabels).toHaveBeenCalledWith('d1', ['l-home', 'l-work']));
      await waitFor(() => expect(within(rowFor(/invoice-march/)).queryByText('Tax')).not.toBeInTheDocument());
      expect(documentService.listFiltered.mock.calls.length).toBe(calls);
    });

    test('deleting the document closes the drawer and reloads the list', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      const drawer = await drawerNamed('lease.pdf');
      documentService.listFiltered.mockResolvedValue(listResponse([DOCS[0], DOCS[2]], 2));
      await user.click(within(drawer).getByRole('button', { name: 'More actions' }));
      await user.click(await screen.findByRole('menuitem', { name: 'Delete document' }));
      await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      await waitFor(() => expect(screen.queryByRole('rowheader', { name: /lease\.pdf/ })).not.toBeInTheDocument());
      expect(currentUrl()).toBe('/documents');
    });

    test('bulk-deleting the open document closes the drawer', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await screen.findByRole('rowheader', { name: /lease\.pdf/ });
      await user.click(within(rowFor(/lease\.pdf/)).getByRole('checkbox'));
      await openWithEnter(user, /lease\.pdf/);
      await drawerNamed('lease.pdf');
      // The bar sits behind the modal drawer; press it without a pointer (as assistive tech can).
      const bar = screen.getByRole('toolbar', { name: 'Bulk actions', hidden: true });
      fireEvent.click(within(bar).getByRole('button', { name: 'Delete', hidden: true }));
      const confirm = await screen.findByRole('alertdialog');
      documentService.listFiltered.mockResolvedValue(listResponse([DOCS[0], DOCS[2]], 2));
      await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
      await waitFor(() => expect(documentService.bulkDelete).toHaveBeenCalledWith(['d2']));
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      await settle();
    });
  });

  describe('changed rows', () => {
    test('a changed row shows its tag and loses it once opened', async () => {
      const user = userEvent.setup();
      markLit('document', 'd2', 'new');
      markLit('document', 'd3', 'changed');
      renderLibrary();
      await screen.findByRole('rowheader', { name: /lease\.pdf/ });
      expect(rowFor(/lease\.pdf/)).toHaveAttribute('data-changed', 'true');
      expect(within(rowFor(/lease\.pdf/)).getByText('New')).toBeInTheDocument();
      expect(within(rowFor(/photo\.png/)).getByText('Changed')).toBeInTheDocument();

      await openWithEnter(user, /lease\.pdf/);
      await drawerNamed('lease.pdf');
      await waitFor(() => expect(isLit('document', 'd2')).toBe(false));
      await user.keyboard('{Escape}');
      await waitFor(() => expect(within(rowFor(/lease\.pdf/)).queryByText('New')).not.toBeInTheDocument());
      expect(within(rowFor(/photo\.png/)).getByText('Changed')).toBeInTheDocument();
    });

    test('"Mark all seen" appears with lit rows and clears them', async () => {
      const user = userEvent.setup();
      markLit('document', 'd2', 'new');
      markLit('document', 'd3', 'changed');
      renderLibrary();
      await screen.findByRole('rowheader', { name: /lease\.pdf/ });
      await user.click(screen.getByRole('button', { name: 'Mark all seen' }));
      expect(isLit('document', 'd2')).toBe(false);
      expect(isLit('document', 'd3')).toBe(false);
      await waitFor(() => expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument());
    });

    test('moving to a row with the arrows also clears its tag', async () => {
      const user = userEvent.setup();
      markLit('document', 'd2', 'changed');
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await drawerNamed('invoice-march.pdf');
      await user.keyboard('{ArrowDown}');
      await drawerNamed('lease.pdf');
      await waitFor(() => expect(isLit('document', 'd2')).toBe(false));
      await settle();
    });
  });

  describe('from Search', () => {
    test('a result opens the drawer over the results and finds the search words in the text', async () => {
      const user = userEvent.setup();
      searchService.enhancedSearch.mockResolvedValue(searchResponse([hit(DOCS[0], 'Invoice for March', [[0, 7]])]));
      renderLibrary('/search?q=total');
      const link = await screen.findByRole('link', { name: 'invoice-march.pdf' });
      expect(link).toHaveAttribute('href', '/search?q=total&document=d1');
      await user.click(link);
      const drawer = await drawerNamed('invoice-march.pdf');
      expect(currentUrl()).toBe('/search?q=total&document=d1');
      await within(drawer).findByText(/120 EUR/);
      expect(Array.from(drawer.querySelectorAll('mark')).map((m) => m.textContent)).toEqual(['Total']);
    });
  });
});
