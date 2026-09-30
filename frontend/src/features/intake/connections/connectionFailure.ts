import i18n from 'i18next';
import { humanizeFailureReason, type HumanFailure } from '../../../lib/failureReason';

interface Pattern {
  test: RegExp;
  key: string;
  fallback: string;
}

/**
 * What a sync or connection test says when a server, bucket or folder can't be reached. The
 * shared document patterns read "access denied" as a file permission problem; for a connection
 * it means the sign-in was refused, so these are checked first.
 */
const PATTERNS: readonly Pattern[] = [
  {
    test: /empty sync ratio|found no (new )?files/i,
    key: 'intake.connectionFailure.emptySyncs',
    fallback: 'Recent syncs found no new files. Check the folder path.',
  },
  {
    test: /\b401\b|\b403\b|unauthori[sz]ed|forbidden|access denied|invalid (credentials|access key)|authentication failed|login failed|signature.*does not match/i,
    key: 'intake.connectionFailure.auth',
    fallback: 'The server refused the sign-in. Check the username, password or keys.',
  },
  {
    test: /\b404\b|not found|no such (bucket|file|directory)|nosuchbucket/i,
    key: 'intake.connectionFailure.notFound',
    fallback: "The folder or bucket wasn't found. Check the address and path.",
  },
  {
    test: /certificate|\bssl\b|\btls\b|handshake/i,
    key: 'intake.connectionFailure.tls',
    fallback: "The server's security certificate isn't trusted.",
  },
  {
    test: /connection refused|could not connect|failed to connect|dns|could not be resolved|name or service not known|no route|unreachable|enotfound|error sending request|timed? ?out|timeout/i,
    key: 'intake.connectionFailure.unreachable',
    fallback: "Can't reach the server. Check the address and that it is online.",
  },
  {
    test: /\b5\d\d\b|internal server error|bad gateway|service unavailable/i,
    key: 'intake.connectionFailure.server',
    fallback: 'The server reported an error. Try again in a few minutes.',
  },
];

/** The health check's advice texts, in plain words. Unknown advice is shown as the server wrote it. */
const ADVICE: readonly Pattern[] = [
  {
    test: /check server url, credentials/i,
    key: 'intake.connectionAdvice.connectivity',
    fallback: 'Check the server address, the sign-in and the network.',
  },
  {
    test: /may indicate connectivity issues|no new content/i,
    key: 'intake.connectionAdvice.noContent',
    fallback: "If new files are expected, check the folder path and that the server is reachable.",
  },
];

function tr(key: string, fallback: string): string {
  const value: unknown = i18n.isInitialized ? i18n.t(key, fallback) : fallback;
  return typeof value === 'string' && value ? value : fallback;
}

/** A health-check recommendation in plain words; the server's text when it isn't a known one. */
export function humanizeAdvice(raw: string): string {
  const text = (raw ?? '').trim();
  const hit = ADVICE.find((p) => p.test.test(text));
  return hit ? tr(hit.key, hit.fallback) : text;
}

/** A connection's last error in one plain sentence; the raw text stays as `detail`. */
export function humanizeConnectionFailure(raw: string, code?: string | null): HumanFailure {
  const text = (raw ?? '').trim();
  const hit = text ? PATTERNS.find((p) => p.test.test(text)) : undefined;
  if (hit) return { summary: tr(hit.key, hit.fallback), detail: text };
  return humanizeFailureReason(text, code);
}
