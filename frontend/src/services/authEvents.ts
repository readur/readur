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
 * When an authenticated request is rejected with 401 the session has expired
 * or been revoked (logout elsewhere, password change, account disabled).
 * Clear the stored token and signal AuthContext so the app returns to /login.
 * 401s from the credential-checking auth endpoints are left to their callers.
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
      if (status === 401 && authHeader && shouldResetSessionOn401(config?.url)) {
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
