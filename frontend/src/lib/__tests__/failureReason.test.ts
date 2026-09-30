import { describe, expect, it } from 'vitest';
import { failureKind, humanizeFailureReason } from '../failureReason';

const DOC_TOOLS =
  'OCR extraction failed: None of the DOC extraction tools (antiword, catdoc, wvText) are available or working.\n\nTried tools: antiword, catdoc, wvText';
const OCR_PATH =
  "OCR extraction failed: OCR failed for '/app/uploads/documents/8d5ce300.pdf' after trying multiple strategies.\n\nExit code: 2\nError output:\nInputFileError\n\nPossible causes and solutions:\n2. Insufficient memory - increase container memory limits";

describe('humanizeFailureReason', () => {
  it('names the missing .doc tools and keeps the raw text as detail', () => {
    expect(humanizeFailureReason(DOC_TOOLS)).toEqual({
      summary: "Can't read .doc files: install antiword or catdoc",
      detail: DOC_TOOLS,
    });
  });

  it('reads an ocrmypdf failure as "OCR failed", even though its advice mentions memory', () => {
    const out = humanizeFailureReason(OCR_PATH);
    expect(out.summary).toBe('OCR failed');
    expect(out.summary).not.toContain('/app/');
    expect(out.detail).toContain('/app/uploads/documents/8d5ce300.pdf');
  });

  it.each([
    ['EncryptedPdfError: input file is encrypted', 'The PDF is password-protected'],
    ['Tesseract timed out after 300s', 'OCR took too long and was stopped'],
    ['Cannot allocate memory', 'Ran out of memory while reading the file'],
    ['Unsupported file type: application/x-foo', "This file type isn't supported"],
    ['File too large: 900 MB', 'The file is too large'],
    ['Permission denied (os error 13)', "Readur isn't allowed to open the file"],
    ['No such file or directory (os error 2)', 'The file is missing from storage'],
    ['Low OCR confidence: 12%', 'The text was too unclear to read reliably'],
    ['PDF parsing error: trailer not found', 'The file looks damaged'],
  ])('maps %j to a plain sentence', (raw, summary) => {
    expect(humanizeFailureReason(raw).summary).toBe(summary);
  });

  it('falls back to the failure code when there is no text', () => {
    expect(humanizeFailureReason('', 'ocr_timeout')).toEqual({ summary: 'OCR took too long and was stopped', detail: '' });
    expect(humanizeFailureReason('', 'duplicate_content')).toEqual({ summary: 'Duplicate content', detail: '' });
  });

  it('says something went wrong when there is nothing at all', () => {
    expect(humanizeFailureReason('')).toEqual({ summary: 'Something went wrong', detail: '' });
    expect(humanizeFailureReason(undefined as unknown as string)).toEqual({ summary: 'Something went wrong', detail: '' });
  });

  it('uses the first line of an unknown message, without prefix or container path', () => {
    const raw = "OCR extraction failed: tool crashed on '/app/uploads/x/scan.pdf'\nstack trace…";
    expect(humanizeFailureReason(raw)).toEqual({ summary: "tool crashed on 'scan.pdf'", detail: raw });
  });

  it('keeps a one-line unknown message as the summary with no duplicate detail', () => {
    expect(humanizeFailureReason('Quota reached')).toEqual({ summary: 'Quota reached', detail: '' });
  });

  it('shortens a very long first line', () => {
    const out = humanizeFailureReason('x'.repeat(300));
    expect(out.summary.length).toBeLessThanOrEqual(120);
    expect(out.summary.endsWith('…')).toBe(true);
  });
});

describe('failureKind', () => {
  it('groups failures by cause', () => {
    expect(failureKind(DOC_TOOLS)).toBe('doc-tools');
    expect(failureKind(OCR_PATH)).toBe('ocr');
    expect(failureKind('', 'pdf_parsing_error')).toBe('damaged');
    expect(failureKind('what?')).toBe('other');
    expect(failureKind(undefined as unknown as string, null)).toBe('other');
  });
});
