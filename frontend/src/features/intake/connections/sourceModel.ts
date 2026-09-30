import type { StatusState } from '../../../ui';
import type { SourceResponse } from '../../../services/api';

type Config = Record<string, unknown>;

export function configOf(source: SourceResponse): Config {
  return source.config && typeof source.config === 'object' && !Array.isArray(source.config)
    ? (source.config as Config)
    : {};
}

/**
 * Connection state for StatusMark: disabled wins, then an active sync, then an error, then the
 * last health check (warning → CHECK, critical → ERROR), else healthy.
 */
export function sourceState(source: SourceResponse): StatusState {
  if (!source.enabled) return 'disabled';
  if (source.status === 'syncing') return 'syncing';
  if (source.status === 'error') return 'error';
  if (source.validation_status === 'critical') return 'error';
  if (source.validation_status === 'warning') return 'warning';
  return 'healthy';
}

export interface ValidationIssue {
  message: string;
  recommendation: string;
  severity: string;
}

/** The health check's findings; the server stores them as a JSON string. */
export function validationIssues(source: SourceResponse): ValidationIssue[] {
  let raw: unknown = (source as { validation_issues?: unknown }).validation_issues;
  if (typeof raw === 'string') {
    try {
      raw = JSON.parse(raw);
    } catch {
      return [];
    }
  }
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((i): i is Record<string, unknown> => Boolean(i) && typeof i === 'object' && typeof (i as { message?: unknown }).message === 'string')
    .map((i) => ({
      message: String(i.message),
      recommendation: typeof i.recommendation === 'string' ? i.recommendation : '',
      severity: typeof i.severity === 'string' ? i.severity : 'warning',
    }));
}

/** The one line that explains why a connection is not healthy, or null when it is. */
export function problemOf(source: SourceResponse): { text: string; tone: 'danger' | 'warning' } | null {
  if (!source.enabled) return null;
  if (source.status === 'error' && source.last_error) return { text: source.last_error, tone: 'danger' };
  const first = validationIssues(source)[0];
  if (first && (source.validation_status === 'critical' || source.validation_status === 'warning')) {
    return { text: first.message, tone: source.validation_status === 'critical' ? 'danger' : 'warning' };
  }
  return null;
}

/** Rows that should be marked for the user: connections whose last sync failed. */
export function isFailing(source: SourceResponse): boolean {
  return source.enabled && source.status === 'error';
}

/** Where the connection points: the server URL, bucket or first folder. */
export function sourceLocation(source: SourceResponse): string {
  const c = configOf(source);
  const folders = Array.isArray(c.watch_folders) ? (c.watch_folders as string[]) : [];
  switch (source.source_type) {
    case 'webdav':
      return typeof c.server_url === 'string' && c.server_url ? c.server_url : '—';
    case 's3': {
      const bucket = typeof c.bucket_name === 'string' ? c.bucket_name : '';
      const endpoint = typeof c.endpoint_url === 'string' && c.endpoint_url ? c.endpoint_url : '';
      return bucket ? (endpoint ? `${bucket} @ ${endpoint}` : bucket) : '—';
    }
    case 'local_folder':
    default:
      return folders.length > 0 ? folders.join(', ') : '—';
  }
}

/** Keeps the first and last characters of a secret and masks the rest. */
export function maskSecret(value: string, keep = 4): string {
  if (!value) return '';
  if (value.length <= keep * 2) return '•'.repeat(Math.max(4, value.length));
  return `${value.slice(0, keep)}••••${value.slice(-keep)}`;
}

/** Auth summary with every secret masked: "alice · password set", "AKIA••••WXYZ", "—". */
export function sourceAuth(source: SourceResponse, passwordSet: string): string {
  const c = configOf(source);
  if (source.source_type === 'webdav') {
    const user = typeof c.username === 'string' ? c.username : '';
    const hasPassword = c.has_password === true || (typeof c.password === 'string' && c.password.length > 0);
    return [user || '—', hasPassword ? passwordSet : null].filter(Boolean).join(' · ');
  }
  if (source.source_type === 's3') {
    const key = typeof c.access_key_id === 'string' ? c.access_key_id : '';
    return key ? maskSecret(key) : '—';
  }
  return '—';
}

export function syncIntervalMinutes(source: SourceResponse): number | null {
  const c = configOf(source);
  if (!c.auto_sync) return null;
  return typeof c.sync_interval_minutes === 'number' && c.sync_interval_minutes > 0 ? c.sync_interval_minutes : null;
}

/** Next scheduled sync (ms since epoch), or null when the connection isn't scheduled. */
export function nextSyncAt(source: SourceResponse, now: number = Date.now()): number | null {
  const interval = syncIntervalMinutes(source);
  if (!source.enabled || interval === null) return null;
  const last = source.last_sync_at ? Date.parse(source.last_sync_at) : NaN;
  if (Number.isNaN(last)) return now;
  return Math.max(now, last + interval * 60_000);
}

export function listOf(source: SourceResponse, key: 'watch_folders' | 'file_extensions'): string[] {
  const value = configOf(source)[key];
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : [];
}
