import React, { createContext, useContext, useEffect, useState } from 'react'
import { api } from '../services/api'
import { AUTH_LOGOUT_EVENT } from '../services/authEvents'

export type UserRole = 'admin' | 'user'

export interface User {
  id: string
  username: string
  email: string
  role: UserRole
  is_active: boolean
}

export interface LoginResponse {
  token: string
  user: User
}

export interface RegisterResult {
  /** True when the account was created but must be approved by an administrator before signing in. */
  pendingApproval: boolean
  user: User
}

interface AuthContextType {
  user: User | null
  loading: boolean
  login: (username: string, password: string) => Promise<void>
  register: (username: string, email: string, password: string) => Promise<RegisterResult>
  logout: () => Promise<void>
  changePassword: (currentPassword: string, newPassword: string) => Promise<void>
  /** Store a session issued by the server (e.g. after the OIDC code exchange). */
  completeLogin: (session: LoginResponse) => void
}

export const isAdmin = (user: { role?: string } | null | undefined): boolean =>
  user?.role === 'admin'

export const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const token = localStorage.getItem('token')
    if (token) {
      api.defaults.headers.common['Authorization'] = `Bearer ${token}`
      fetchUser()
    } else {
      setLoading(false)
    }

    // The API layer signals when the session is no longer valid (e.g. a 401
    // after the token was revoked); drop the in-memory user as well.
    const handleForcedLogout = () => setUser(null)
    window.addEventListener(AUTH_LOGOUT_EVENT, handleForcedLogout)
    return () => window.removeEventListener(AUTH_LOGOUT_EVENT, handleForcedLogout)
  }, [])

  const clearSession = () => {
    localStorage.removeItem('token')
    delete api.defaults.headers.common['Authorization']
    setUser(null)
  }

  const storeSession = ({ token, user: userData }: LoginResponse) => {
    localStorage.setItem('token', token)
    api.defaults.headers.common['Authorization'] = `Bearer ${token}`
    setUser(userData)
  }

  const fetchUser = async () => {
    try {
      const response = await api.get('/auth/me')
      setUser(response.data)
    } catch (error) {
      clearSession()
    } finally {
      setLoading(false)
    }
  }

  const login = async (username: string, password: string) => {
    const response = await api.post<LoginResponse>('/auth/login', { username, password })
    storeSession(response.data)
  }

  const register = async (username: string, email: string, password: string): Promise<RegisterResult> => {
    const response = await api.post<User>('/auth/register', { username, email, password })
    const created = response.data
    if (created && created.is_active === true) {
      // Approval not required on this server: sign straight in.
      await login(username, password)
      return { pendingApproval: false, user: created }
    }
    return { pendingApproval: true, user: created }
  }

  const logout = async () => {
    const token = localStorage.getItem('token')
    clearSession()
    if (token) {
      try {
        // Best effort: revoke all server-side sessions for this user.
        await api.post('/auth/logout', undefined, {
          headers: { Authorization: `Bearer ${token}` },
        })
      } catch {
        // Ignore: local state is already cleared.
      }
    }
  }

  const changePassword = async (currentPassword: string, newPassword: string) => {
    const response = await api.post<LoginResponse>('/auth/password', {
      current_password: currentPassword,
      new_password: newPassword,
    })
    storeSession(response.data)
  }

  const value = {
    user,
    loading,
    login,
    register,
    logout,
    changePassword,
    completeLogin: storeSession,
  }

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
