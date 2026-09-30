import { shortType } from '../library/mime';

/** MIME types that say nothing about the format: the file name's extension decides instead. */
const GENERIC_MIMES = new Set(['', 'application/octet-stream', 'binary/octet-stream', 'application/x-empty', 'application/unknown']);

function extensionOf(name: string | null | undefined): string | null {
  const match = /\.([a-z0-9]{1,5})$/i.exec(name ?? '');
  return match ? match[1].toUpperCase() : null;
}

/**
 * The short type code shown in every TYPE cell (Library, Board, document pass): PDF, DOCX, PNG…
 * The MIME type decides, through the Library's table (so a .docx reads DOCX everywhere); a
 * generic or missing MIME type falls back to the file name's extension; with neither, a dash.
 */
export function typeCodeOf(mimeType: string | null | undefined, fileName?: string | null): string {
  const mime = (mimeType ?? '').split(';')[0].trim().toLowerCase();
  if (!GENERIC_MIMES.has(mime)) return shortType(mime);
  return extensionOf(fileName) ?? '—';
}
