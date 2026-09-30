/**
 * Turns the raw failure text the server stores for a document (a multi-line tool log, often with
 * container paths in it) into one plain sentence a person can act on. The raw text stays
 * available as `detail`, for a "Details" disclosure.
 */
import i18n from 'i18next';

export interface HumanFailure {
  /** One short sentence: what went wrong and, where known, what fixes it. */
  summary: string;
  /** The raw text, trimmed. Empty when it adds nothing to the summary. */
  detail: string;
}

interface Pattern {
  /** Stable id: groups failures with the same cause. */
  id: string;
  test: RegExp;
  key: string;
  fallback: string;
}

/**
 * Checked in order; the first match wins. Specific tool messages come before the generic ones
 * ("OCR failed for '/app/…'" often mentions memory in its advice text, so it must win over it).
 */
const PATTERNS: readonly Pattern[] = [
  {
    id: 'doc-tools',
    test: /None of the DOC extraction tools/i,
    key: 'failure.docTools',
    fallback: "Can't read .doc files: install antiword or catdoc",
  },
  {
    id: 'encrypted',
    test: /EncryptedPdfError|password[- ]protected|is encrypted/i,
    key: 'failure.encrypted',
    fallback: 'The PDF is password-protected',
  },
  { id: 'ocr', test: /OCR failed for (PDF )?'/i, key: 'failure.ocr', fallback: 'OCR failed' },
  { id: 'timeout', test: /timed? ?out|timeout/i, key: 'failure.timeout', fallback: 'OCR took too long and was stopped' },
  {
    id: 'memory',
    test: /out of memory|memory limit|cannot allocate/i,
    key: 'failure.memory',
    fallback: 'Ran out of memory while reading the file',
  },
  {
    id: 'unsupported',
    test: /unsupported (file )?(format|type)|not supported/i,
    key: 'failure.unsupported',
    fallback: "This file type isn't supported",
  },
  { id: 'too-large', test: /too large|exceeds? .*size/i, key: 'failure.tooLarge', fallback: 'The file is too large' },
  {
    id: 'permission',
    test: /permission denied|access denied/i,
    key: 'failure.permission',
    fallback: "Readur isn't allowed to open the file",
  },
  {
    id: 'missing',
    test: /no such file|file not found/i,
    key: 'failure.missing',
    fallback: 'The file is missing from storage',
  },
  {
    id: 'low-confidence',
    test: /low (ocr )?confidence/i,
    key: 'failure.lowConfidence',
    fallback: 'The text was too unclear to read reliably',
  },
  {
    id: 'damaged',
    test: /corrupt|malformed|pdf parsing|invalid (pdf|structure)/i,
    key: 'failure.damaged',
    fallback: 'The file looks damaged',
  },
];

/** The server's failure codes (`ocr_failure_reason`), for when there is no message to read. */
const CODES: Record<string, string> = {
  low_ocr_confidence: 'low-confidence',
  ocr_timeout: 'timeout',
  timeout: 'timeout',
  ocr_memory_limit: 'memory',
  pdf_parsing_error: 'damaged',
  pdf_corruption: 'damaged',
  file_corrupted: 'damaged',
  invalid_structure: 'damaged',
  unsupported_format: 'unsupported',
  file_too_large: 'too-large',
  access_denied: 'permission',
  permission_denied: 'permission',
};

const MAX_SUMMARY = 120;

/** Translated text, or the English fallback before i18n is ready. */
function tr(key: string, fallback: string): string {
  const value: unknown = i18n.isInitialized ? i18n.t(key, fallback) : fallback;
  return typeof value === 'string' && value ? value : fallback;
}

/** "OCR extraction failed: X" → "X"; a container path is cut down to its file name. */
function firstLine(raw: string): string {
  const line = raw.split('\n').find((l) => l.trim()) ?? '';
  const cleaned = line
    .replace(/^(OCR extraction failed|Error|error):\s*/i, '')
    .replace(/'(\/[^']*\/)([^'/]+)'/g, "'$2'")
    .trim();
  return cleaned.length > MAX_SUMMARY ? `${cleaned.slice(0, MAX_SUMMARY - 1).trimEnd()}…` : cleaned;
}

function match(raw: string, code?: string | null): Pattern | undefined {
  const byText = raw ? PATTERNS.find((p) => p.test.test(raw)) : undefined;
  if (byText) return byText;
  const id = code ? CODES[code] : undefined;
  return id ? PATTERNS.find((p) => p.id === id) : undefined;
}

/**
 * A stable id for the cause of a failure (for grouping), or `other` when no known pattern
 * matches.
 */
export function failureKind(raw: string, code?: string | null): string {
  return match(raw ?? '', code)?.id ?? 'other';
}

/**
 * Plain-language summary of a raw failure text. `code` is the server's failure code, used only
 * when the text itself matches nothing known.
 */
export function humanizeFailureReason(raw: string, code?: string | null): HumanFailure {
  const text = (raw ?? '').trim();
  const pattern = match(text, code);
  if (pattern) return { summary: tr(pattern.key, pattern.fallback), detail: text };
  if (!text) {
    const readable = (code ?? '').replace(/_/g, ' ').trim();
    const summary = readable
      ? readable.charAt(0).toUpperCase() + readable.slice(1)
      : tr('failure.unknown', 'Something went wrong');
    return { summary, detail: '' };
  }
  const summary = firstLine(text) || tr('failure.unknown', 'Something went wrong');
  return { summary, detail: summary === text ? '' : text };
}
