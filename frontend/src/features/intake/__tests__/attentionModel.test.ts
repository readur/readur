import { describe, expect, it } from 'vitest';
import i18n from 'i18next';
import './intakeTestUtils';
import type { FailedDocumentRow } from '../../../services/api';
import {
  FAILURE_REASONS,
  FAILURE_STAGES,
  canRetry,
  confidenceText,
  failedName,
  failureSummary,
  reasonLabel,
  stageLabel,
} from '../attention/failureLabels';
import { ATTENTION_VIEWS } from '../attention/AttentionSection';
import { formatBytes, formatDateTime, formatMinutes, formatRelative, humanize, shortHash } from '../shared/format';
import { failedDoc } from './intakeTestUtils';

const t = i18n.t.bind(i18n);
const row = (o: Record<string, unknown> = {}) => failedDoc('d1', o) as unknown as FailedDocumentRow;

describe('attention helpers (ported from DocumentManagementPage.patterns)', () => {
  it('reads the failure summary null-safely from any row shape', () => {
    expect(failureSummary(t, row())).toBe('Timeout');
    expect(failureSummary(t, row({ failure_category: undefined, failure_reason: 'file_too_large' }))).toBe('File too large');
    expect(failureSummary(t, row({ failure_category: '', failure_reason: 'brand_new_reason' }))).toBe('brand new reason');
  });

  it('falls back sensibly when optional values are missing', () => {
    expect(failedName(row({ original_filename: null }))).toBe('d1.pdf');
    expect(canRetry(row())).toBe(true);
    expect(canRetry(row({ can_retry: false }))).toBe(false);
    expect(formatBytes(null)).toBe('0 B');
    expect(formatDateTime(null)).toBe('—');
    expect(formatRelative(undefined)).toBe('—');
    expect(formatMinutes(undefined)).toBe('—');
  });

  it('matches the failed-documents response shape it reads', () => {
    const doc = row();
    for (const key of ['id', 'filename', 'failure_reason', 'failure_stage', 'error_message', 'retry_count', 'created_at', 'updated_at']) {
      expect(doc).toHaveProperty(key);
    }
  });

  it('formats confidence and other values safely', () => {
    expect(confidenceText(42.46)).toBe('42.5%');
    expect(confidenceText(0)).toBe('0.0%');
    expect(confidenceText(null)).toBeNull();
    expect(confidenceText(undefined)).toBeNull();
    expect(confidenceText(Number.NaN)).toBeNull();
    expect(humanize('low_ocr_confidence')).toBe('low ocr confidence');
    expect(shortHash('abcdef0123456789abcdef')).toBe('abcdef0123456789…');
    expect(shortHash('abc')).toBe('abc');
  });

  it('keeps the view ids and filter options stable', () => {
    expect(ATTENTION_VIEWS).toEqual(['failed', 'lowConfidence', 'duplicates', 'cleanup']);
    expect(FAILURE_STAGES).toEqual(['ocr', 'ingestion', 'validation', 'storage', 'processing', 'sync']);
    expect(FAILURE_REASONS).toContain('low_ocr_confidence');
    for (const stage of FAILURE_STAGES) expect(stageLabel(t, stage)).not.toMatch(/_/);
    for (const reason of FAILURE_REASONS) expect(reasonLabel(t, reason)).not.toMatch(/_/);
  });
});
