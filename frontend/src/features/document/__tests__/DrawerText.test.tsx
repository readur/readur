import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { POLL_INTERVAL_MS } from '../hooks/useDocument';
import { primeApi, type ApiMock } from './mockApi';
import { makeDocument, makeOcr, renderDrawer, stubClipboard, stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());
vi.mock('../../board/litStore', () => ({ acknowledge: vi.fn() }));
vi.mock('../../../components/RetryHistoryModal', () => ({ RetryHistoryModal: () => null }));
vi.mock('../../labels/LabelSelector', () => ({ default: () => null }));

const m = apiModule as unknown as ApiMock;

function load(doc = makeDocument(), ocr = makeOcr()) {
  m.documentService.getById.mockResolvedValue({ data: doc });
  m.documentService.getOcrText.mockResolvedValue({ data: ocr });
}

const title = () => screen.findByRole('dialog', { name: 'invoice.pdf' });
const textSection = () => screen.getByRole('region', { name: 'Text' });
const textBody = () => screen.findByRole('region', { name: 'Extracted text' });

beforeEach(() => {
  stubObjectUrls();
  primeApi(m);
});

describe('extracted text: word count and stats', () => {
  it('shows a word count of 0', async () => {
    load(makeDocument({ ocr_word_count: 0 }), makeOcr({ ocr_word_count: 0, ocr_text: '' }));
    renderDrawer();
    await title();
    await waitFor(() => expect(m.documentService.getOcrText).toHaveBeenCalled());
    expect(await within(textSection()).findByText(/(^|· )0 words/)).toBeInTheDocument();
  });

  it('shows no word count when it is null', async () => {
    load(makeDocument({ ocr_word_count: undefined }), makeOcr({ ocr_word_count: null as unknown as number }));
    renderDrawer();
    await title();
    await textBody();
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  it('shows no word count when it is missing', async () => {
    const ocr = makeOcr();
    delete ocr.ocr_word_count;
    load(makeDocument({ ocr_word_count: undefined }), ocr);
    renderDrawer();
    await title();
    await textBody();
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  it('shows confidence, words and processing time in one line', async () => {
    load();
    renderDrawer();
    await title();
    await textBody();
    const stats = within(textSection()).getByText(/confidence/);
    expect(stats).toHaveTextContent('96% confidence');
    expect(stats).toHaveTextContent('290 words');
    expect(stats).toHaveTextContent('1,500 ms');
  });

  it('says so when OCR found no text', async () => {
    load(makeDocument(), makeOcr({ ocr_text: '' }));
    renderDrawer();
    await title();
    expect(await within(textSection()).findByText('No text was found in this document.')).toBeInTheDocument();
  });

  it('shows the status while OCR is still pending', async () => {
    load(makeDocument({ ocr_status: 'pending', has_ocr_text: false }));
    renderDrawer();
    await title();
    expect(within(textSection()).getByText('The text appears here when OCR finishes.')).toBeInTheDocument();
    expect(within(textSection()).getByText('Pending')).toBeInTheDocument();
    expect(m.documentService.getOcrText).not.toHaveBeenCalled();
  });

  it('copies all text', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load();
    renderDrawer();
    await title();
    await textBody();
    await user.click(screen.getByRole('button', { name: 'Copy all text' }));
    expect(writeText).toHaveBeenCalledWith(makeOcr().ocr_text);
    expect(await screen.findByText('Text copied')).toBeInTheDocument();
  });

  it('switches to monospace from the display options and remembers it', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    await textBody();
    await user.click(screen.getByRole('button', { name: 'Text display options' }));
    const option = await screen.findByRole('menuitemcheckbox', { name: 'Monospace' });
    expect(option).toHaveAttribute('aria-checked', 'false');
    await user.click(option);
    expect(option).toHaveAttribute('aria-checked', 'true');
    expect(window.localStorage.getItem('readur.document.mono')).toBe('1');
    await user.click(option);
    expect(option).toHaveAttribute('aria-checked', 'false');
    expect(window.localStorage.getItem('readur.document.mono')).toBe('0');
  });
});

describe('extracted text: reading layout', () => {
  const raw = 'Report\n\n\n\nLine one\nLine two\n\n   \n\nLast part\n';

  it('turns runs of blank lines into single paragraph gaps and keeps line breaks', async () => {
    load(makeDocument(), makeOcr({ ocr_text: raw }));
    renderDrawer();
    await title();
    const body = await textBody();
    const paragraphs = Array.from(body.querySelectorAll('p')).map((p) => p.textContent);
    expect(paragraphs).toEqual(['Report', 'Line one\nLine two', 'Last part']);
  });

  it('still copies the raw text, blank lines and all', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load(makeDocument(), makeOcr({ ocr_text: raw }));
    renderDrawer();
    await title();
    await textBody();
    await user.click(screen.getByRole('button', { name: 'Copy all text' }));
    expect(writeText).toHaveBeenCalledWith(raw);
  });

  it('reports a failed copy', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    writeText.mockRejectedValue(new Error('denied'));
    load();
    renderDrawer();
    await title();
    await textBody();
    await user.click(screen.getByRole('button', { name: 'Copy all text' }));
    expect(await screen.findByText("Couldn't copy the text")).toBeInTheDocument();
  });

  it('explains a failed OCR in plain words, with the raw message behind a disclosure', async () => {
    load(makeDocument({ ocr_status: 'failed', has_ocr_text: false }));
    m.documentService.getDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'doc-1',
        total_retries: 1,
        retry_history: [
          {
            id: 'r1',
            retry_reason: 'manual',
            priority: 10,
            created_at: '2025-06-16T11:00:00Z',
            previous_error: 'Tesseract timed out\nstage: ocr\npath: /app/uploads/x.pdf',
          },
        ],
      },
    });
    renderDrawer();
    await title();
    const section = textSection();
    expect(await within(section).findByText('OCR took too long and was stopped')).toBeInTheDocument();
    expect(within(section).getByText('Show the full message')).toBeInTheDocument();
  });

  it('says so when the text could not be loaded', async () => {
    load();
    m.documentService.getOcrText.mockRejectedValue(new Error('down'));
    renderDrawer();
    await title();
    expect(await within(textSection()).findByText("Couldn't load the extracted text.")).toBeInTheDocument();
  });
});

describe('extracted text: find', () => {
  it('highlights every match and reports the position', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    const body = await textBody();
    await user.type(screen.getByRole('searchbox', { name: 'Find in text' }), 'invoice');
    const marks = body.querySelectorAll('mark');
    expect(Array.from(marks).map((mk) => mk.textContent)).toEqual(['Invoice', 'invoice']);
    expect(screen.getByText('1 of 2')).toBeInTheDocument();
    expect(marks[0]).toHaveAttribute('data-current', 'true');

    await user.click(screen.getByRole('button', { name: 'Next match' }));
    expect(screen.getByText('2 of 2')).toBeInTheDocument();
    expect(body.querySelectorAll('mark')[1]).toHaveAttribute('data-current', 'true');

    await user.keyboard('{Enter}');
    expect(screen.getByText('1 of 2')).toBeInTheDocument();

    await user.keyboard('{Shift>}{Enter}{/Shift}');
    expect(screen.getByText('2 of 2')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Previous match' }));
    expect(screen.getByText('1 of 2')).toBeInTheDocument();
  });

  it('clears the find field with its clear button', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    const body = await textBody();
    const field = screen.getByRole('searchbox', { name: 'Find in text' });
    expect(screen.queryByRole('button', { name: 'Clear' })).not.toBeInTheDocument();
    await user.type(field, 'invoice');
    await user.click(screen.getByRole('button', { name: 'Clear' }));
    expect(field).toHaveValue('');
    expect(body.querySelector('mark')).toBeNull();
  });

  it('matches the words of a search one by one when the phrase is not in the text', async () => {
    load(makeDocument(), makeOcr({ ocr_text: 'Injury to the left shoulder. The shoulder healed.' }));
    renderDrawer({ path: '/search?q=shoulder injury&document=doc-1' });
    await title();
    const body = await textBody();
    expect(Array.from(body.querySelectorAll('mark')).map((mk) => mk.textContent)).toEqual([
      'Injury',
      'shoulder',
      'shoulder',
    ]);
    expect(screen.getByText('1 of 3')).toBeInTheDocument();
  });

  it('says when nothing matches', async () => {
    const user = userEvent.setup();
    load();
    renderDrawer();
    await title();
    const body = await textBody();
    await user.type(screen.getByRole('searchbox', { name: 'Find in text' }), 'zebra');
    expect(screen.getByText('No matches')).toBeInTheDocument();
    expect(body.querySelector('mark')).toBeNull();
    expect(screen.getByRole('button', { name: 'Next match' })).toBeDisabled();
  });

  it('treats the search text literally', async () => {
    const user = userEvent.setup();
    load(makeDocument(), makeOcr({ ocr_text: 'Price (net): 5.00 / (gross) 6.00' }));
    renderDrawer();
    await title();
    const body = await textBody();
    await user.type(screen.getByRole('searchbox', { name: 'Find in text' }), '(net)');
    expect(Array.from(body.querySelectorAll('mark')).map((mk) => mk.textContent)).toEqual(['(net)']);
  });

  it('prefills the find field from ?q= and scrolls to the first match', async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    load();
    renderDrawer({ path: '/search?q=total&document=doc-1' });
    await title();
    const body = await textBody();
    expect(screen.getByRole('searchbox', { name: 'Find in text' })).toHaveValue('total');
    const marks = body.querySelectorAll('mark');
    expect(marks).toHaveLength(1);
    expect(marks[0]).toHaveTextContent('Total');
    await waitFor(() => expect(scroll).toHaveBeenCalled());
    expect(scroll.mock.contexts[0]).toBe(marks[0]);
  });
});

describe('document polling', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it.each(['pending', 'processing'])('re-reads the document every 10s while OCR is %s', async (status) => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    load(makeDocument({ ocr_status: status, has_ocr_text: false }));
    renderDrawer();
    await waitFor(() => expect(m.documentService.getById).toHaveBeenCalledTimes(1));
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS - 100);
    });
    expect(m.documentService.getById).toHaveBeenCalledTimes(1);
    await act(async () => {
      vi.advanceTimersByTime(100);
    });
    await waitFor(() => expect(m.documentService.getById).toHaveBeenCalledTimes(2));
  });

  it('does not poll a completed document', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    load();
    renderDrawer();
    await waitFor(() => expect(m.documentService.getById).toHaveBeenCalledTimes(1));
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS * 3);
    });
    expect(m.documentService.getById).toHaveBeenCalledTimes(1);
  });

  it('stops polling and loads the text once OCR finishes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    m.documentService.getById
      .mockResolvedValueOnce({ data: makeDocument({ ocr_status: 'processing', has_ocr_text: false }) })
      .mockResolvedValue({ data: makeDocument() });
    m.documentService.getOcrText.mockResolvedValue({ data: makeOcr() });
    renderDrawer();
    await title();
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent('OCR');
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS);
    });
    expect(await textBody()).toHaveTextContent('Invoice 42');
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent('Indexed');
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS * 3);
    });
    expect(m.documentService.getById).toHaveBeenCalledTimes(2);
  });
});
