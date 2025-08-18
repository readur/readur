/**
 * Mock handlers for authentication-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockUser,
  getDefaultTestUser,
  getDefaultAdminUser,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
  createErrorConfig,
} from '../utils/config'
import { MockConfig, MockUser } from '../api/types'

// Mock state for authentication
let mockUsers: MockUser[] = [getDefaultTestUser(), getDefaultAdminUser()]
let currentUser: MockUser | null = null
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all auth handlers
 */
export const setAuthMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Set current authenticated user
 */
export const setCurrentUser = (user: MockUser | null) => {
  currentUser = user
}

/**
 * Set mock users data
 */
export const setMockUsers = (users: MockUser[]) => {
  mockUsers = users
}

/**
 * Reset auth state to default
 */
export const resetAuthState = () => {
  mockUsers = [getDefaultTestUser(), getDefaultAdminUser()]
  currentUser = null
  mockConfig = DEFAULT_MOCK_CONFIG
}

/**
 * Generate a mock JWT token
 */
const generateMockToken = (user: MockUser): string => {
  const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }))
  const payload = btoa(JSON.stringify({
    sub: user.id,
    username: user.username,
    role: user.role,
    exp: Math.floor(Date.now() / 1000) + 3600, // 1 hour
  }))
  const signature = btoa('mock-signature')
  return `${header}.${payload}.${signature}`
}

export const authHandlers = [
  // POST /api/auth/login - User login
  http.post('/api/auth/login', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { username: string; password: string }

    // Find user by username
    const user = mockUsers.find(u => 
      u.username === body.username && u.is_active
    )

    if (!user || body.password !== 'password') {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('UNAUTHORIZED', 'Invalid credentials')),
        { status: 401 }
      )
    }

    currentUser = user
    const token = generateMockToken(user)

    return HttpResponse.json(createMockResponse({
      token,
      user: {
        id: user.id,
        username: user.username,
        email: user.email,
        role: user.role,
      },
    }, mockConfig))
  }),

  // POST /api/auth/register - User registration
  http.post('/api/auth/register', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { username: string; email: string; password: string }

    // Check if user already exists
    const existingUser = mockUsers.find(u => 
      u.username === body.username || u.email === body.email
    )

    if (existingUser) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('CONFLICT', 'User already exists')),
        { status: 409 }
      )
    }

    // Create new user
    const newUser = createMockUser({
      username,
      email,
      role: 'user',
      is_active: true,
    })

    mockUsers.push(newUser)
    currentUser = newUser
    const token = generateMockToken(newUser)

    return HttpResponse.json(createMockResponse({
      token,
      user: {
        id: newUser.id,
        username: newUser.username,
        email: newUser.email,
        role: newUser.role,
      },
    }, mockConfig))
  }),

  // POST /api/auth/logout - User logout
  http.post('/api/auth/logout', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    currentUser = null

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Logged out successfully',
    }, mockConfig))
  }),

  // GET /api/auth/me - Get current user
  http.get('/api/auth/me', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const authHeader = request.headers.get('Authorization')
    
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('UNAUTHORIZED', 'No token provided')),
        { status: 401 }
      )
    }

    if (!currentUser) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('UNAUTHORIZED', 'Invalid token')),
        { status: 401 }
      )
    }

    return HttpResponse.json(createMockResponse({
      id: currentUser.id,
      username: currentUser.username,
      email: currentUser.email,
      role: currentUser.role,
      created_at: currentUser.created_at,
      is_active: currentUser.is_active,
    }, mockConfig))
  }),

  // POST /api/auth/refresh - Refresh token
  http.post('/api/auth/refresh', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const authHeader = request.headers.get('Authorization')
    
    if (!authHeader || !authHeader.startsWith('Bearer ') || !currentUser) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('UNAUTHORIZED', 'Invalid token')),
        { status: 401 }
      )
    }

    const newToken = generateMockToken(currentUser)

    return HttpResponse.json(createMockResponse({
      token: newToken,
      user: {
        id: currentUser.id,
        username: currentUser.username,
        email: currentUser.email,
        role: currentUser.role,
      },
    }, mockConfig))
  }),

  // POST /api/auth/forgot-password - Forgot password
  http.post('/api/auth/forgot-password', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { email: string }

    const user = mockUsers.find(u => u.email === body.email)

    // Always return success for security reasons
    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'If an account with that email exists, a reset link has been sent.',
    }, mockConfig))
  }),

  // POST /api/auth/reset-password - Reset password
  http.post('/api/auth/reset-password', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { token: string; password: string }

    // Mock token validation - in real implementation, would validate JWT
    if (!body.token || body.token.length < 10) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('BAD_REQUEST', 'Invalid reset token')),
        { status: 400 }
      )
    }

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Password reset successfully',
    }, mockConfig))
  }),

  // GET /api/auth/oidc/config - Get OIDC configuration
  http.get('/api/auth/oidc/config', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse({
      enabled: true,
      provider_name: 'Mock OIDC Provider',
      authorization_url: 'https://auth.example.com/authorize',
      client_id: 'mock-client-id',
      scopes: ['openid', 'profile', 'email'],
    }, mockConfig))
  }),

  // POST /api/auth/oidc/callback - OIDC callback
  http.post('/api/auth/oidc/callback', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { code: string; state: string }

    if (!body.code) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('BAD_REQUEST', 'Authorization code required')),
        { status: 400 }
      )
    }

    // Find or create OIDC user
    let user = mockUsers.find(u => u.oidc_sub)
    if (!user) {
      user = createMockUser({
        username: 'oidc_user',
        email: 'oidc@example.com',
        role: 'user',
        is_active: true,
        oidc_sub: 'oidc|mock-sub-123',
      })
      mockUsers.push(user)
    }

    currentUser = user
    const token = generateMockToken(user)

    return HttpResponse.json(createMockResponse({
      token,
      user: {
        id: user.id,
        username: user.username,
        email: user.email,
        role: user.role,
      },
    }, mockConfig))
  }),
]