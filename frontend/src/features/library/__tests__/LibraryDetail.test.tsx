import { describe, test, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { isLit, markLit } from '../../board/litStore';
import { currentUrl, renderLibrary, settle } from './libraryTestUtils';
import { DOCS, documentService, hit, labelService, searchResponse, searchService, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());

type User = ReturnType<typeof userEvent.setup>;

// `hidden`: while the panel is open, the page behind it is hidden from assistive tech.
const rowFor = (name: RegExp) =>
  screen.getByRole('rowheader', { name, hidden: true }).closest('[role="row"]') as HTMLElement;
const panel = () => screen.getByRole('dialog');

async function openWithEnter(user: User, name: RegExp) {
  await screen.findByRole('rowheader', { name });
  const row = rowFor(name);
  act(() => row.focus());
  await user.keyboard('{Enter}');
  return screen.findByRole('dialog');
}

describe('Library detail panel', () => {
  beforeEach(() => {
    setupLibraryMocks();
  });

  describe('opening and moving', () => {
    test('Enter on a row opens the panel for that document', async () => {
      const user = userEvent.setup();
      renderLibrary();
      const dialog = await openWithEnter(user, /invoice-march/);
      expect(within(dialog).getByRole('heading', { name: /invoice-march\.pdf/ })).toBeInTheDocument();
      await settle();
    });

    test('clicking a row opens the panel without selecting it', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await user.click(await screen.findByRole('rowheader', { name: /lease\.pdf/ }));
      expect(await screen.findByRole('heading', { name: /lease\.pdf/ })).toBeInTheDocument();
      expect(screen.queryByRole('toolbar', { name: 'Bulk actions' })).not.toBeInTheDocument();
      await settle();
    });

    test('↓ and ↑ move to the next and previous document', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await user.keyboard('{ArrowDown}');
      expect(within(panel()).getByRole('heading', { name: /lease\.pdf/ })).toBeInTheDocument();
      await user.keyboard('{ArrowDown}');
      expect(within(panel()).getByRole('heading', { name: /photo\.png/ })).toBeInTheDocument();
      await user.keyboard('{ArrowUp}');
      expect(within(panel()).getByRole('heading', { name: /lease\.pdf/ })).toBeInTheDocument();
      await settle();
    });

    test('the arrows stop at the ends of the page', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await user.keyboard('{ArrowUp}');
      expect(within(panel()).getByRole('heading', { name: /invoice-march/ })).toBeInTheDocument();
      await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}');
      expect(within(panel()).getByRole('heading', { name: /photo\.png/ })).toBeInTheDocument();
      await settle();
    });

    test('Escape closes the panel and returns focus to the row', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      await waitFor(() => expect(rowFor(/lease\.pdf/)).toHaveFocus());
    });
  });

  describe('content', () => {
    test('shows the document facts', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      const facts = await within(panel()).findByRole('group', { name: 'Document facts' });
      const value = (term: string) => within(facts).getByText(term).nextElementSibling;
      expect(value('Type')).toHaveTextContent('PDF');
      expect(value('Size')).toHaveTextContent('2.0 KB');
      expect(value('Source')).toHaveTextContent('Upload');
      await waitFor(() => expect(value('Pages/OCR')).toHaveTextContent('2'));
      expect(within(facts).getByText('Language').nextElementSibling).toHaveTextContent('eng');
      expect(value('Confidence')).toHaveTextContent('91%');
      expect(value('Added')).toHaveTextContent('2026');
      expect(value('Updated')).toHaveTextContent('2026');
    });

    test('shows the OCR status in the header', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /photo\.png/);
      expect(within(panel()).getByRole('heading', { name: /OCR 3\/12/ })).toBeInTheDocument();
      await settle();
    });

    test('shows the start of the OCR text', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      expect(await within(panel()).findByText(/Total due: 120 EUR/)).toBeInTheDocument();
      expect(documentService.getOcrText).toHaveBeenCalledWith('d1');
    });

    test('marks search matches in the OCR text', async () => {
      const user = userEvent.setup();
      searchService.enhancedSearch.mockResolvedValue(searchResponse([hit(DOCS[0], 'Invoice for March', [[0, 7]])]));
      renderLibrary('/documents?q=total');
      await openWithEnter(user, /invoice-march/);
      await within(panel()).findByText(/120 EUR/);
      const marks = Array.from(panel().querySelectorAll('mark')).map((m) => m.textContent);
      expect(marks).toEqual(['Total']);
    });

    test('says so when there is no text yet', async () => {
      const user = userEvent.setup();
      documentService.getOcrText.mockResolvedValue({ data: { id: 'd3', ocr_text: null, has_ocr_text: false } });
      renderLibrary();
      await openWithEnter(user, /photo\.png/);
      expect(await within(panel()).findByText('No text has been read from this document yet.')).toBeInTheDocument();
    });

    test('only fetches the text once per document', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await within(panel()).findByText(/120 EUR/);
      await user.keyboard('{ArrowDown}');
      await user.keyboard('{ArrowUp}');
      await within(panel()).findByText(/120 EUR/);
      expect(documentService.getOcrText.mock.calls.filter(([id]) => id === 'd1')).toHaveLength(1);
    });
  });

  describe('labels', () => {
    test('editing labels saves at once and confirms', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('combobox', { name: 'Labels' }));
      await user.click(await screen.findByRole('option', { name: /tax/i }));
      await waitFor(() => expect(labelService.setDocumentLabels).toHaveBeenCalledWith('d2', ['l-tax']));
      expect(await screen.findByText('Labels saved')).toBeInTheDocument();
      expect(within(rowFor(/lease\.pdf/)).getByText('Tax')).toBeInTheDocument();
    });

    test('removing a label saves the rest', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await user.click(within(panel()).getByRole('button', { name: 'Remove Home' }));
      await waitFor(() => expect(labelService.setDocumentLabels).toHaveBeenCalledWith('d1', ['l-tax', 'l-work']));
      await settle();
    });

    test('a failed save puts the labels back and says so', async () => {
      const user = userEvent.setup();
      labelService.setDocumentLabels.mockRejectedValue(new Error('nope'));
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('combobox', { name: 'Labels' }));
      await user.click(await screen.findByRole('option', { name: /tax/i }));
      expect(await screen.findByText('Could not save labels')).toBeInTheDocument();
      expect(within(rowFor(/lease\.pdf/)).queryByText('Tax')).not.toBeInTheDocument();
    });
  });

  describe('actions', () => {
    test('Open goes to the document page', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('button', { name: 'Open' }));
      expect(currentUrl()).toBe('/documents/d2');
    });

    test('Download downloads the file under its name', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('button', { name: 'Download' }));
      expect(documentService.downloadFile).toHaveBeenCalledWith('d2', 'lease.pdf');
      await settle();
    });

    test('Retry OCR is offered only for failed documents', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      expect(within(panel()).queryByRole('button', { name: 'Retry OCR' })).not.toBeInTheDocument();
      await user.keyboard('{ArrowDown}');
      await user.click(within(panel()).getByRole('button', { name: 'Retry OCR' }));
      expect(documentService.retryOcr).toHaveBeenCalledWith('d2');
      expect(await screen.findByText('Text recognition queued again')).toBeInTheDocument();
    });

    test('Share leads to the document’s sharing', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('button', { name: 'Share' }));
      expect(currentUrl()).toBe('/documents/d2?share=1');
    });

    test('Delete asks first and cancelling keeps the document', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('button', { name: 'Delete' }));
      const confirm = await screen.findByRole('alertdialog', { name: 'Delete this document?' });
      expect(within(confirm).getByText(/“lease\.pdf”/)).toBeInTheDocument();
      await user.click(within(confirm).getByRole('button', { name: 'Cancel' }));
      expect(documentService.delete).not.toHaveBeenCalled();
      await settle();
    });

    test('confirming Delete removes the document and closes the panel', async () => {
      const user = userEvent.setup();
      renderLibrary();
      await openWithEnter(user, /lease\.pdf/);
      await user.click(within(panel()).getByRole('button', { name: 'Delete' }));
      const confirm = await screen.findByRole('alertdialog');
      await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
      expect(documentService.delete).toHaveBeenCalledWith('d2');
      await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
      expect(documentService.listFiltered.mock.calls.length).toBeGreaterThan(1);
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
      expect(within(rowFor(/lease\.pdf/)).getByText('NEW')).toBeInTheDocument();
      expect(within(rowFor(/photo\.png/)).getByText('CHANGED')).toBeInTheDocument();
      expect(rowFor(/invoice-march/)).not.toHaveAttribute('data-changed');

      await openWithEnter(user, /lease\.pdf/);
      expect(isLit('document', 'd2')).toBe(false);
      await user.keyboard('{Escape}');
      await waitFor(() => expect(within(rowFor(/lease\.pdf/)).queryByText('NEW')).not.toBeInTheDocument());
      expect(rowFor(/lease\.pdf/)).not.toHaveAttribute('data-changed');
      expect(within(rowFor(/photo\.png/)).getByText('CHANGED')).toBeInTheDocument();
    });

    test('moving to a row with the arrows also clears its tag', async () => {
      const user = userEvent.setup();
      markLit('document', 'd2', 'changed');
      renderLibrary();
      await openWithEnter(user, /invoice-march/);
      await user.keyboard('{ArrowDown}');
      expect(isLit('document', 'd2')).toBe(false);
      await settle();
    });
  });
});
