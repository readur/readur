import { ErrorCodes, ErrorHelper, type ErrorCode } from '../../../services/errors';

export { ErrorCodes };

export function hasCode(error: unknown, code: ErrorCode): boolean {
  try {
    return ErrorHelper.isErrorCode(error, code);
  } catch {
    return false;
  }
}

export function statusOf(error: unknown): number | undefined {
  return (error as { response?: { status?: number } } | null)?.response?.status;
}

/** Server message from an API error body, if any. */
export function serverMessage(error: unknown): string | undefined {
  const data = (error as { response?: { data?: { message?: unknown; error?: unknown } } } | null)?.response?.data;
  const text = data?.message ?? data?.error;
  return typeof text === 'string' && text.trim() ? text : undefined;
}

export type ErrorCategory = 'auth' | 'validation' | 'network' | 'server' | 'unknown';

export function categoryOf(error: unknown): ErrorCategory {
  try {
    return ErrorHelper.getErrorCategory(error);
  } catch {
    return 'unknown';
  }
}

/** First matching message for a list of `[code, message]` pairs, else the server message, else `fallback`. */
export function pickMessage(
  error: unknown,
  pairs: ReadonlyArray<readonly [ErrorCode, string]>,
  fallback: string,
): string {
  for (const [code, message] of pairs) {
    if (hasCode(error, code)) return message;
  }
  return serverMessage(error) ?? fallback;
}
