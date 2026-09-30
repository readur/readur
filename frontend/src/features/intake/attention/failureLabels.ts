import type { TFunction } from 'i18next';
import type { FailedDocumentRow } from '../../../services/api';
import { humanize } from '../shared/format';

export const FAILURE_STAGES = ['ocr', 'ingestion', 'validation', 'storage', 'processing', 'sync'] as const;

export const FAILURE_REASONS = [
  'duplicate_content',
  'low_ocr_confidence',
  'unsupported_format',
  'file_too_large',
  'file_corrupted',
  'ocr_timeout',
  'pdf_parsing_error',
  'other',
] as const;

export function stageLabel(t: TFunction, stage: string): string {
  switch (stage) {
    case 'ocr':
      return t('intake.attention.stage.ocr', 'OCR');
    case 'ingestion':
      return t('intake.attention.stage.ingestion', 'Ingestion');
    case 'validation':
      return t('intake.attention.stage.validation', 'Validation');
    case 'storage':
      return t('intake.attention.stage.storage', 'Storage');
    case 'processing':
      return t('intake.attention.stage.processing', 'Processing');
    case 'sync':
      return t('intake.attention.stage.sync', 'Sync');
    default:
      return humanize(stage);
  }
}

export function reasonLabel(t: TFunction, reason: string): string {
  switch (reason) {
    case 'duplicate_content':
      return t('intake.attention.reason.duplicate_content', 'Duplicate content');
    case 'low_ocr_confidence':
      return t('intake.attention.reason.low_ocr_confidence', 'Low OCR confidence');
    case 'unsupported_format':
      return t('intake.attention.reason.unsupported_format', 'Unsupported format');
    case 'file_too_large':
      return t('intake.attention.reason.file_too_large', 'File too large');
    case 'file_corrupted':
      return t('intake.attention.reason.file_corrupted', 'File corrupted');
    case 'ocr_timeout':
      return t('intake.attention.reason.ocr_timeout', 'OCR timed out');
    case 'pdf_parsing_error':
      return t('intake.attention.reason.pdf_parsing_error', 'PDF could not be read');
    case 'other':
      return t('intake.attention.reason.other', 'Other');
    default:
      return humanize(reason);
  }
}

/** The failure category the server computed, else the reason in words. */
export function failureSummary(t: TFunction, doc: FailedDocumentRow): string {
  return doc.failure_category || reasonLabel(t, doc.failure_reason);
}

export const failedName = (doc: FailedDocumentRow): string => doc.original_filename || doc.filename;

/** A missing `can_retry` (the list endpoint never sends it) means retryable. */
export const canRetry = (doc: FailedDocumentRow): boolean => doc.can_retry !== false;

/** Confidence as "42.5%", or null when unknown. Null-safe for rows without OCR results. */
export function confidenceText(value: number | null | undefined): string | null {
  return typeof value === 'number' && Number.isFinite(value) ? `${value.toFixed(1)}%` : null;
}
