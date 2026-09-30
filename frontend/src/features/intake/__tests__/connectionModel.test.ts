import { describe, expect, it } from 'vitest';
import i18n from 'i18next';
import type { SourceResponse } from '../../../services/api';
import {
  isFailing,
  maskSecret,
  nextSyncAt,
  sourceAuth,
  sourceLocation,
  sourceState,
} from '../connections/sourceModel';
import {
  SOURCE_ERROR_SEVERITIES,
  SOURCE_ERROR_TYPES,
  errorSourceTypeOf,
  errorTypeLabel,
  severityLabel,
  severityState,
} from '../connections/sourceErrorLabels';
import { source } from './intakeTestUtils';

const s = (o: Record<string, unknown> = {}) => source('s1', o) as unknown as SourceResponse;
const t = i18n.t.bind(i18n);

describe('connection status mapping', () => {
  it.each([
    ['disabled wins over everything', { enabled: false, status: 'error' }, 'disabled'],
    ['syncing', { status: 'syncing' }, 'syncing'],
    ['sync error', { status: 'error' }, 'error'],
    ['critical health check', { validation_status: 'critical' }, 'error'],
    ['health warning', { validation_status: 'warning' }, 'warning'],
    ['healthy', { validation_status: 'healthy' }, 'healthy'],
    ['idle with no health check', {}, 'healthy'],
  ])('%s', (_name, overrides, state) => {
    expect(sourceState(s(overrides))).toBe(state);
  });

  it('marks only enabled connections whose sync failed', () => {
    expect(isFailing(s({ status: 'error' }))).toBe(true);
    expect(isFailing(s({ status: 'error', enabled: false }))).toBe(false);
    expect(isFailing(s({ status: 'idle' }))).toBe(false);
  });
});

describe('connection details', () => {
  it('shows the server URL, bucket or folders as the location', () => {
    expect(sourceLocation(s())).toBe('https://cloud.example.com');
    expect(sourceLocation(s({ source_type: 's3', config: { bucket_name: 'docs', endpoint_url: 'https://minio' } }))).toBe('docs @ https://minio');
    expect(sourceLocation(s({ source_type: 'local_folder', config: { watch_folders: ['/a', '/b'] } }))).toBe('/a, /b');
  });

  it('never shows a password or secret in the sign-in summary', () => {
    const webdav = sourceAuth(s(), 'password set');
    expect(webdav).toBe('ada · password set');
    expect(webdav).not.toContain('secret');
    const s3 = sourceAuth(s({ source_type: 's3', config: { access_key_id: 'AKIAABCDEFGHWXYZ', secret_access_key: 'topsecret' } }), 'x');
    expect(s3).toBe('AKIA••••WXYZ');
    expect(maskSecret('short')).toBe('•••••');
  });

  it('computes the next scheduled sync only for enabled automatic connections', () => {
    const now = Date.parse('2026-05-01T12:00:00Z');
    const last = '2026-05-01T11:30:00Z';
    expect(nextSyncAt(s({ last_sync_at: last }), now)).toBe(Date.parse('2026-05-01T12:30:00Z'));
    expect(nextSyncAt(s({ config: { auto_sync: false } }), now)).toBeNull();
    expect(nextSyncAt(s({ enabled: false }), now)).toBeNull();
    expect(nextSyncAt(s({ last_sync_at: null }), now)).toBe(now);
  });
});

describe('source error enums (PascalCase on the wire)', () => {
  it('has a label for every generated error type', () => {
    for (const type of SOURCE_ERROR_TYPES) {
      const label = errorTypeLabel(t, type);
      expect(label).toBeTruthy();
      expect(label).not.toBe(type.toLowerCase());
    }
    expect(errorTypeLabel(t, 'PermissionDenied')).toBe('Permission denied');
    expect(errorTypeLabel(t, 'Timeout')).toBe('Timeout');
  });

  it('does not treat the old lowercase values as known types', () => {
    expect(errorTypeLabel(t, 'timeout')).toBe('timeout');
    expect(severityLabel(t, 'high')).toBe('high');
  });

  it('maps severities to status marks: High and Critical are errors, Low and Medium are checks', () => {
    expect(SOURCE_ERROR_SEVERITIES.map(severityState)).toEqual(['warning', 'warning', 'error', 'error']);
    expect(severityLabel(t, 'Critical')).toBe('Critical');
  });

  it('maps connection types to the error tracker source types', () => {
    expect(errorSourceTypeOf('webdav')).toBe('WebDAV');
    expect(errorSourceTypeOf('s3')).toBe('S3');
    expect(errorSourceTypeOf('local_folder')).toBe('Local');
  });
});
