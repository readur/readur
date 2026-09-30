/** The one file-type helper: short type codes (PDF, DOCX, PNG…) shared by Library, Board and document views. */

const SHORT: Record<string, string> = {
  'application/pdf': 'PDF',
  'image/jpeg': 'JPEG',
  'image/png': 'PNG',
  'image/tiff': 'TIFF',
  'image/gif': 'GIF',
  'image/webp': 'WEBP',
  'image/bmp': 'BMP',
  'image/svg+xml': 'SVG',
  'text/plain': 'TXT',
  'text/csv': 'CSV',
  'text/markdown': 'MD',
  'text/html': 'HTML',
  'application/json': 'JSON',
  'application/xml': 'XML',
  'application/rtf': 'RTF',
  'application/msword': 'DOC',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'DOCX',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'XLSX',
  'application/vnd.openxmlformats-officedocument.presentationml.presentation': 'PPTX',
  'application/vnd.ms-excel': 'XLS',
  'application/vnd.ms-powerpoint': 'PPT',
  'application/vnd.oasis.opendocument.text': 'ODT',
  'application/vnd.oasis.opendocument.spreadsheet': 'ODS',
};

/** Short uppercase type for the TYPE column, e.g. PDF, PNG, DOCX. */
export function shortType(mime: string | null | undefined): string {
  if (!mime) return '—';
  const known = SHORT[mime.toLowerCase()];
  if (known) return known;
  const sub = mime.split('/')[1] ?? mime;
  const tail = sub.split(/[.+-]/).filter(Boolean).pop() ?? sub;
  return tail.slice(0, 6).toUpperCase();
}

/** MIME types that say nothing about the format: the file name's extension decides instead. */
const GENERIC_MIMES = new Set(['', 'application/octet-stream', 'binary/octet-stream', 'application/x-empty', 'application/unknown']);

function extensionOf(name: string | null | undefined): string | null {
  const match = /\.([a-z0-9]{1,5})$/i.exec(name ?? '');
  return match ? match[1].toUpperCase() : null;
}

/**
 * The short type code shown in every TYPE cell (Library, Board, document summary): PDF, DOCX, PNG…
 * The MIME type decides, through the Library's table (so a .docx reads DOCX everywhere); a
 * generic or missing MIME type falls back to the file name's extension; with neither, a dash.
 */
export function typeCodeOf(mimeType: string | null | undefined, fileName?: string | null): string {
  const mime = (mimeType ?? '').split(';')[0].trim().toLowerCase();
  if (!GENERIC_MIMES.has(mime)) return shortType(mime);
  return extensionOf(fileName) ?? '—';
}
