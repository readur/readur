import type { TFunction } from 'i18next';
import type { Document, OcrResponse } from '../../services/api';
import { typeCodeOf } from '../../lib/fileType';
import { formatBytes, ocrState, sourceLabel, type OcrExtras } from './format';

/** "30 Sep 2026 08:04" in the reader's language and time zone, or null for a missing date. */
export function formatAdded(value: string | null | undefined, lng?: string): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  const day = date.toLocaleDateString(lng, { day: 'numeric', month: 'short', year: 'numeric' });
  const time = date.toLocaleTimeString(lng, { hour: '2-digit', minute: '2-digit' });
  return `${day} ${time}`;
}

/**
 * Page count from OCR, which reads the pages one by one, or from OCR progress while it runs.
 * `source_metadata.page_count` is not used: the server derives it by counting "/Type /Page" in
 * the raw file, which also matches the "/Type /Pages" tree node, so a 1-page PDF reports 2.
 */
export function pageCount(doc: Document, ocr: OcrResponse | null): number | null {
  const fromOcr = (ocr as OcrExtras | null)?.pages_processed;
  if (typeof fromOcr === 'number' && fromOcr > 0) return fromOcr;
  const total = doc.ocr_progress_total ?? 0;
  return total > 0 ? total : null;
}

export interface MetaInput {
  document: Document;
  ocr: OcrResponse | null;
  /** Name of the connection the file came through, when it has one. */
  sourceName?: string | null;
  t: TFunction;
  lng?: string;
}

/**
 * The document's facts as short phrases for one line: "PDF · 2 pages · 2.1 KB · Upload ·
 * Added 30 Sep 2026 08:04 · OCR 85%". Values that are unknown are left out, never shown as "—".
 */
export function metaParts({ document: doc, ocr, sourceName, t, lng }: MetaInput): string[] {
  const parts: string[] = [typeCodeOf(doc.mime_type, doc.original_filename || doc.filename)];
  const pages = pageCount(doc, ocr);
  if (pages) parts.push(t('document.meta.pages', { count: pages, defaultValue: pages === 1 ? '{{count}} page' : '{{count}} pages' }));
  if (doc.file_size != null && Number.isFinite(doc.file_size)) parts.push(formatBytes(doc.file_size));
  parts.push(sourceName || sourceLabel(doc.source_type, (key, fallback) => t(key, fallback)));
  const added = formatAdded(doc.created_at, lng);
  if (added) parts.push(t('document.meta.added', { date: added, defaultValue: 'Added {{date}}' }));
  const language = (ocr as OcrExtras | null)?.detected_language;
  if (language) parts.push(language.toUpperCase());
  const confidence = ocr?.ocr_confidence ?? doc.ocr_confidence;
  if (confidence != null && ocrState(doc.ocr_status) === 'completed') {
    parts.push(t('document.meta.ocr', { value: Math.round(confidence), defaultValue: 'OCR {{value}}%' }));
  }
  return parts;
}
