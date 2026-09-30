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
