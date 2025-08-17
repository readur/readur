/**
 * MockAuthProvider - Authentication state management for testing
 * Provides complete mock authentication context with realistic state transitions
 */

import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react'
import { useMockAuth } from '../../utils/react-hooks'
import { createMockUser, createMockUserWithScenario } from '../../factories/user'
import type { MockUser } from '../../api/types'

export interface MockAuthContextType {
  // Authentication state
  user: MockUser | null
  isAuthenticated: boolean
  isLoading: boolean
  
  // Authentication actions
  login: (user?: MockUser | 'admin' | 'user', options?: LoginOptions) => Promise<void>
  logout: () => Promise<void>
  register: (userData?: Partial<MockUser>) => Promise<void>
  
  // User management
  setUser: (user: MockUser | null) => void
  updateUser: (updates: Partial<MockUser>) => void
  
  // Authentication scenarios
  simulateLoginDelay: (delay?: number) => void
  simulateAuthError: (errorType?: 'invalid_credentials' | 'network_error' | 'server_error') => void
  simulateTokenExpiry: () => void
  simulateAccountLock: () => void
  
  // State utilities
  reset: () => void
  getCurrentAuthState: () => AuthState
}

export interface LoginOptions {
  rememberMe?: boolean
  delay?: number
  shouldFail?: boolean
  errorType?: 'invalid_credentials' | 'network_error' | 'server_error'
}

export interface AuthState {
  user: MockUser | null
  isAuthenticated: boolean
  isLoading: boolean
  lastLogin: Date | null
  sessionId: string | null
  permissions: string[]
}

const MockAuthContext = createContext<MockAuthContextType | null>(null)

export interface MockAuthProviderProps {
  children: ReactNode
  initialUser?: MockUser | 'admin' | 'user' | null
  autoLogin?: boolean
  simulateLoadingStates?: boolean
  enableSessionManagement?: boolean
  defaultPermissions?: string[]
}

export const MockAuthProvider: React.FC<MockAuthProviderProps> = ({
  children,
  initialUser = null,
  autoLogin = false,
  simulateLoadingStates = true,
  enableSessionManagement = true,
  defaultPermissions = ['read', 'write'],
}) => {
  const [authError, setAuthError] = useState<string | null>(null)
  const [loginDelay, setLoginDelay] = useState(0)
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [lastLogin, setLastLogin] = useState<Date | null>(null)
  const [permissions, setPermissions] = useState<string[]>(defaultPermissions)
  
  const mockAuth = useMockAuth()

  // Setup initial user if provided
  useEffect(() => {
    if (initialUser && autoLogin) {
      let user: MockUser
      
      if (typeof initialUser === 'string') {
        user = initialUser === 'admin' 
          ? createMockUserWithScenario('admin_user')
          : createMockUserWithScenario('typical_user')
      } else {
        user = initialUser
      }
      
      mockAuth.login(user)
      if (enableSessionManagement) {
        setSessionId(`session_${Date.now()}`)
        setLastLogin(new Date())
      }
    }
  }, [initialUser, autoLogin, mockAuth, enableSessionManagement])

  const login = useCallback(async (
    userOrType?: MockUser | 'admin' | 'user',
    options: LoginOptions = {}
  ) => {
    const { rememberMe = false, delay = loginDelay, shouldFail = false, errorType } = options
    
    setAuthError(null)
    
    if (simulateLoadingStates) {
      mockAuth.setUser(null)
      // Simulate loading state during delay
    }

    if (delay > 0) {
      await new Promise(resolve => setTimeout(resolve, delay))
    }

    if (shouldFail) {
      const errorMessage = getAuthErrorMessage(errorType || 'invalid_credentials')
      setAuthError(errorMessage)
      throw new Error(errorMessage)
    }

    let user: MockUser

    if (!userOrType) {
      user = createMockUser()
    } else if (typeof userOrType === 'string') {
      user = userOrType === 'admin' 
        ? createMockUserWithScenario('admin_user')
        : createMockUserWithScenario('typical_user')
    } else {
      user = userOrType
    }

    // Set user permissions based on role
    const userPermissions = getUserPermissions(user.role || 'user')
    setPermissions(userPermissions)

    if (enableSessionManagement) {
      setSessionId(`session_${Date.now()}_${user.id}`)
      setLastLogin(new Date())
      
      if (rememberMe) {
        localStorage.setItem('mock_remember_user', JSON.stringify(user))
      }
    }

    mockAuth.login(user)
  }, [mockAuth, loginDelay, simulateLoadingStates, enableSessionManagement])

  const logout = useCallback(async () => {
    if (simulateLoadingStates) {
      // Brief loading state for logout
      await new Promise(resolve => setTimeout(resolve, 100))
    }

    mockAuth.logout()
    setAuthError(null)
    setSessionId(null)
    setLastLogin(null)
    setPermissions(defaultPermissions)
    
    if (enableSessionManagement) {
      localStorage.removeItem('mock_remember_user')
    }
  }, [mockAuth, simulateLoadingStates, enableSessionManagement, defaultPermissions])

  const register = useCallback(async (userData: Partial<MockUser> = {}) => {
    setAuthError(null)
    
    if (simulateLoadingStates) {
      await new Promise(resolve => setTimeout(resolve, 500))
    }

    const newUser = createMockUser({
      ...userData,
      id: userData.id || `new_user_${Date.now()}`,
      created_at: new Date().toISOString(),
    })

    await login(newUser)
  }, [login, simulateLoadingStates])

  const updateUser = useCallback((updates: Partial<MockUser>) => {
    if (mockAuth.currentUser) {
      const updatedUser = { ...mockAuth.currentUser, ...updates }
      mockAuth.setUser(updatedUser)
    }
  }, [mockAuth])

  const simulateLoginDelay = useCallback((delay: number = 2000) => {
    setLoginDelay(delay)
  }, [])

  const simulateAuthError = useCallback((errorType: 'invalid_credentials' | 'network_error' | 'server_error' = 'invalid_credentials') => {
    const errorMessage = getAuthErrorMessage(errorType)
    setAuthError(errorMessage)
  }, [])

  const simulateTokenExpiry = useCallback(() => {
    setAuthError('Session expired. Please log in again.')
    mockAuth.logout()
    setSessionId(null)
  }, [mockAuth])

  const simulateAccountLock = useCallback(() => {
    setAuthError('Account temporarily locked due to multiple failed login attempts.')
    mockAuth.logout()
  }, [mockAuth])

  const reset = useCallback(() => {
    mockAuth.logout()
    setAuthError(null)
    setLoginDelay(0)
    setSessionId(null)
    setLastLogin(null)
    setPermissions(defaultPermissions)
    
    if (enableSessionManagement) {
      localStorage.removeItem('mock_remember_user')
    }
  }, [mockAuth, defaultPermissions, enableSessionManagement])

  const getCurrentAuthState = useCallback((): AuthState => ({
    user: mockAuth.currentUser,
    isAuthenticated: mockAuth.isAuthenticated,
    isLoading: false, // You could track this with additional state
    lastLogin,
    sessionId,
    permissions,
  }), [mockAuth.currentUser, mockAuth.isAuthenticated, lastLogin, sessionId, permissions])

  const contextValue: MockAuthContextType = {
    user: mockAuth.currentUser,
    isAuthenticated: mockAuth.isAuthenticated,
    isLoading: false, // Could be enhanced with loading state tracking
    login,
    logout,
    register,
    setUser: mockAuth.setUser,
    updateUser,
    simulateLoginDelay,
    simulateAuthError,
    simulateTokenExpiry,
    simulateAccountLock,
    reset,
    getCurrentAuthState,
  }

  return (
    <MockAuthContext.Provider value={contextValue}>
      {children}
      {authError && (
        <div 
          data-testid="auth-error" 
          style={{ 
            position: 'fixed', 
            top: 0, 
            left: 0, 
            right: 0, 
            background: '#fee', 
            color: '#c33', 
            padding: '8px', 
            textAlign: 'center',
            zIndex: 9999 
          }}
        >
          {authError}
        </div>
      )}
    </MockAuthContext.Provider>
  )
}

export const useMockAuthContext = (): MockAuthContextType => {
  const context = useContext(MockAuthContext)
  if (!context) {
    throw new Error('useMockAuthContext must be used within a MockAuthProvider')
  }
  return context
}

// Utility functions
function getAuthErrorMessage(errorType: string): string {
  switch (errorType) {
    case 'invalid_credentials':
      return 'Invalid username or password'
    case 'network_error':
      return 'Network error. Please check your connection.'
    case 'server_error':
      return 'Server error. Please try again later.'
    default:
      return 'Authentication failed'
  }
}

function getUserPermissions(role: string): string[] {
  switch (role) {
    case 'admin':
      return ['read', 'write', 'delete', 'admin', 'manage_users', 'manage_settings']
    case 'editor':
      return ['read', 'write', 'delete']
    case 'user':
    default:
      return ['read', 'write']
  }
}

// High-order component for wrapping components with mock auth
export const withMockAuth = <P extends object>(
  Component: React.ComponentType<P>,
  providerProps?: Partial<MockAuthProviderProps>
) => {
  const WrappedComponent = (props: P) => (
    <MockAuthProvider {...providerProps}>
      <Component {...props} />
    </MockAuthProvider>
  )
  
  WrappedComponent.displayName = `withMockAuth(${Component.displayName || Component.name})`
  return WrappedComponent
}