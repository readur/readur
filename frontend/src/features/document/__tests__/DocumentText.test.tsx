import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { POLL_INTERVAL_MS } from '../hooks/useDocument';
import { primeApi, type ApiMock } from './mockApi';
import { makeDocument, makeOcr, renderPage, setViewport, stubClipboard, stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());
vi.mock('../../board/litStore', () => ({ acknowledge: vi.fn() }));
vi.mock('../../../components/RetryHistoryModal', () => ({ RetryHistoryModal: () => null }));
vi.mock('../../labels/LabelSelector', () => ({ default: () => null }));

const m = apiModule as unknown as ApiMock;

function load(doc = makeDocument(), ocr = makeOcr()) {
  m.documentService.getById.mockResolvedValue({ data: doc });
  m.documentService.getOcrText.mockResolvedValue({ data: ocr });
}

const title = () => screen.findByRole('heading', { level: 1, name: 'invoice.pdf' });
const textSection = () => screen.getByRole('region', { name: 'Text' });
const textBody = () => screen.findByRole('region', { name: 'Extracted text' });

beforeEach(() => {
  setViewport(true);
  stubObjectUrls();
  primeApi(m);
});

describe('extracted text: word count and stats', () => {
  it('shows a word count of 0', async () => {
    load(makeDocument({ ocr_word_count: 0 }), makeOcr({ ocr_word_count: 0, ocr_text: '' }));
    renderPage();
    await title();
    await waitFor(() => expect(m.documentService.getOcrText).toHaveBeenCalled());
    expect(await within(textSection()).findByText(/(^|· )0 words/)).toBeInTheDocument();
  });

  it('shows no word count when it is null', async () => {
    load(makeDocument({ ocr_word_count: undefined }), makeOcr({ ocr_word_count: null as unknown as number }));
    renderPage();
    await title();
    await textBody();
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  it('shows no word count when it is missing', async () => {
    const ocr = makeOcr();
    delete ocr.ocr_word_count;
    load(makeDocument({ ocr_word_count: undefined }), ocr);
    renderPage();
    await title();
    await textBody();
    expect(screen.queryByText(/\d+ words/i)).not.toBeInTheDocument();
  });

  it('shows confidence, words and processing time in one line', async () => {
    load();
    renderPage();
    await title();
    await textBody();
    const stats = within(textSection()).getByText(/confidence/);
    expect(stats).toHaveTextContent('96% confidence');
    expect(stats).toHaveTextContent('290 words');
    expect(stats).toHaveTextContent('1,500 ms');
  });

  it('says so when OCR found no text', async () => {
    load(makeDocument(), makeOcr({ ocr_text: '' }));
    renderPage();
    await title();
    expect(await within(textSection()).findByText('No text was found in this document.')).toBeInTheDocument();
  });

  it('shows the status while OCR is still pending', async () => {
    load(makeDocument({ ocr_status: 'pending', has_ocr_text: false }));
    renderPage();
    await title();
    expect(within(textSection()).getByText('The text appears here when OCR finishes.')).toBeInTheDocument();
    expect(within(textSection()).getByText('PENDING')).toBeInTheDocument();
    expect(m.documentService.getOcrText).not.toHaveBeenCalled();
  });

  it('copies all text', async () => {
    const user = userEvent.setup();
    const writeText = stubClipboard();
    load();
    renderPage();
    await title();
    await textBody();
    await user.click(screen.getByRole('button', { name: 'Copy all text' }));
    expect(writeText).toHaveBeenCalledWith(makeOcr().ocr_text);
    expect(await screen.findByText('Text copied')).toBeInTheDocument();
  });

  it('switches between readable and monospace text', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
    await title();
    await textBody();
    const toggle = screen.getByRole('switch', { name: 'Monospace' });
    expect(toggle).not.toBeChecked();
    await user.click(toggle);
    expect(toggle).toBeChecked();
  });
});

describe('extracted text: find', () => {
  it('highlights every match and reports the position', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
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
  });

  it('says when nothing matches', async () => {
    const user = userEvent.setup();
    load();
    renderPage();
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
    renderPage();
    await title();
    const body = await textBody();
    await user.type(screen.getByRole('searchbox', { name: 'Find in text' }), '(net)');
    expect(Array.from(body.querySelectorAll('mark')).map((mk) => mk.textContent)).toEqual(['(net)']);
  });

  it('prefills the find field from ?q= and scrolls to the first match', async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    load();
    renderPage({ path: '/documents/doc-1?q=total' });
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
    renderPage();
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
    renderPage();
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
    renderPage();
    await title();
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent('OCR');
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS);
    });
    expect(await textBody()).toHaveTextContent('Invoice 42');
    expect(screen.getByRole('group', { name: 'Document summary' })).toHaveTextContent('INDEXED');
    await act(async () => {
      vi.advanceTimersByTime(POLL_INTERVAL_MS * 3);
    });
    expect(m.documentService.getById).toHaveBeenCalledTimes(2);
  });
});
