import type { AxiosInstance } from 'axios'

/**
 * Dispatched on `window` when the API layer detects that the current session
 * is no longer valid (e.g. a 401 on an authenticated request). AuthContext
 * listens for it and drops the signed-in user, which routes back to /login.
 */
export const AUTH_LOGOUT_EVENT = 'readur:auth-logout'

/** Auth endpoints whose 401 responses are expected and handled by the caller. */
const AUTH_ENDPOINTS_WITHOUT_SESSION_RESET = [
  '/auth/login',
  '/auth/register',
  '/auth/password',
  '/auth/logout',
  '/auth/oidc/exchange',
]

export function shouldResetSessionOn401(url: string | undefined): boolean {
  if (!url) return true
  const path = url.split('?')[0]
  return !AUTH_ENDPOINTS_WITHOUT_SESSION_RESET.some(
    (endpoint) => path === endpoint || path.endsWith(endpoint)
  )
}

/**
 * True when the rejected request was sent with the session that is still
 * stored. A 401 for an older token (e.g. a request that was in flight while
 * the user signed in again) must not end the newer session.
 */
export function isCurrentSessionToken(authHeader: unknown): boolean {
  const token = localStorage.getItem('token')
  return typeof authHeader === 'string' && !!token && authHeader === `Bearer ${token}`
}

/**
 * Tokens being replaced by an in-flight request that issues a new session
 * (password change). The server revokes the old token before the response
 * carrying the new one arrives, so concurrent requests with the old token can
 * be rejected in between; those 401s must not end the session.
 */
const rotatingTokens = new Set<string>()

function isRotatingToken(authHeader: unknown): boolean {
  return (
    typeof authHeader === 'string' &&
    authHeader.startsWith('Bearer ') &&
    rotatingTokens.has(authHeader.slice('Bearer '.length))
  )
}

/**
 * Run `rotate`, which replaces the stored session token, without letting 401s
 * for the current token end the session meanwhile. If the rotation fails the
 * guard is lifted, so a token that really was revoked still ends the session
 * on its next rejected request.
 */
export async function withSessionRotation<T>(rotate: () => Promise<T>): Promise<T> {
  const token = localStorage.getItem('token')
  if (token) rotatingTokens.add(token)
  try {
    return await rotate()
  } finally {
    if (token) rotatingTokens.delete(token)
  }
}

/**
 * When an authenticated request is rejected with 401 the session has expired
 * or been revoked (logout elsewhere, password change, account disabled).
 * Clear the stored token and signal AuthContext so the app returns to /login.
 * 401s from the credential-checking auth endpoints are left to their callers,
 * and so are 401s for a token other than the one currently stored or for a
 * token that is being replaced (see withSessionRotation).
 */
const instrumented = new WeakSet<object>()

export function installSessionInterceptor(instance: AxiosInstance): void {
  if (!instance || instrumented.has(instance)) return
  instrumented.add(instance)
  instance.interceptors?.response?.use(
    (response) => response,
    (error) => {
      const status = error?.response?.status
      const config = error?.config
      const authHeader = config?.headers?.Authorization ?? config?.headers?.authorization
      if (
        status === 401 &&
        shouldResetSessionOn401(config?.url) &&
        isCurrentSessionToken(authHeader) &&
        !isRotatingToken(authHeader)
      ) {
        localStorage.removeItem('token')
        delete instance.defaults.headers.common['Authorization']
        if (typeof window !== 'undefined') {
          window.dispatchEvent(new Event(AUTH_LOGOUT_EVENT))
        }
      }
      return Promise.reject(error)
    }
  )
}
