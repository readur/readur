/**
 * useMockUser - User state management for testing
 * Provides comprehensive user management with realistic authentication flows
 */

import { useState, useCallback, useEffect } from 'react'
import { useMockAuthContext } from '../providers/MockAuthProvider'
import { createMockUser, createMockUserWithScenario } from '../../factories/user'
import type { MockUser } from '../../api/types'

export interface UserProfile extends MockUser {
  preferences?: UserPreferences
  statistics?: UserStatistics
  permissions?: string[]
  sessionInfo?: SessionInfo
}

export interface UserPreferences {
  theme: 'light' | 'dark' | 'auto'
  language: string
  timezone: string
  notifications: NotificationPreferences
  dashboard: DashboardPreferences
}

export interface NotificationPreferences {
  email: boolean
  browser: boolean
  mobile: boolean
  types: string[]
}

export interface DashboardPreferences {
  layout: 'grid' | 'list'
  documentsPerPage: number
  defaultSort: string
  showPreview: boolean
}

export interface UserStatistics {
  documentsUploaded: number
  searchesPerformed: number
  lastLoginAt: string
  totalSessionTime: number
  favoriteFeatures: string[]
}

export interface SessionInfo {
  sessionId: string
  loginTime: Date
  lastActivity: Date
  ipAddress: string
  userAgent: string
  isActive: boolean
}

export interface UseMockUserReturn {
  // Current user state
  user: UserProfile | null
  isAuthenticated: boolean
  isLoading: boolean
  
  // User management
  loginAs: (userType: UserType | MockUser) => Promise<void>
  logout: () => Promise<void>
  switchUser: (userId: string) => Promise<void>
  
  // Profile management
  updateProfile: (updates: Partial<UserProfile>) => void
  updatePreferences: (preferences: Partial<UserPreferences>) => void
  resetPreferences: () => void
  
  // User scenarios
  simulateNewUser: () => Promise<void>
  simulateExperiencedUser: () => Promise<void>
  simulateAdminUser: () => Promise<void>
  simulateGuestUser: () => Promise<void>
  
  // Authentication scenarios
  simulateSessionExpiry: () => void
  simulateAccountLock: () => void
  simulatePasswordReset: () => Promise<void>
  simulateOIDCLogin: () => Promise<void>
  
  // User behavior simulation
  simulateUserActivity: (actions: UserAction[]) => void
  simulateInactivity: (duration: number) => void
  simulateMultipleLogins: () => void
  
  // Statistics and tracking
  recordActivity: (action: string, metadata?: any) => void
  getActivityHistory: () => ActivityRecord[]
  getUserInsights: () => UserInsights
  
  // Testing utilities
  createTestUser: (overrides?: Partial<MockUser>) => MockUser
  cloneCurrentUser: () => MockUser | null
  reset: () => void
}

export type UserType = 'admin' | 'editor' | 'user' | 'guest' | 'new_user' | 'experienced_user'

export interface UserAction {
  type: 'upload' | 'search' | 'download' | 'delete' | 'view' | 'edit'
  target?: string
  metadata?: any
  delay?: number
}

export interface ActivityRecord {
  timestamp: Date
  action: string
  metadata?: any
  sessionId: string
}

export interface UserInsights {
  mostUsedFeatures: string[]
  averageSessionDuration: number
  productivityScore: number
  usagePatterns: string[]
  recommendations: string[]
}

const DEFAULT_PREFERENCES: UserPreferences = {
  theme: 'light',
  language: 'en',
  timezone: 'UTC',
  notifications: {
    email: true,
    browser: true,
    mobile: false,
    types: ['upload_complete', 'ocr_failed', 'system_updates'],
  },
  dashboard: {
    layout: 'grid',
    documentsPerPage: 20,
    defaultSort: 'created_at',
    showPreview: true,
  },
}

export const useMockUser = (): UseMockUserReturn => {
  const authContext = useMockAuthContext?.()
  
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null)
  const [activityHistory, setActivityHistory] = useState<ActivityRecord[]>([])
  const [sessionInfo, setSessionInfo] = useState<SessionInfo | null>(null)

  // Sync with auth context
  useEffect(() => {
    if (authContext?.user && authContext.user !== userProfile) {
      const enhancedUser: UserProfile = {
        ...authContext.user,
        preferences: DEFAULT_PREFERENCES,
        statistics: {
          documentsUploaded: Math.floor(Math.random() * 100),
          searchesPerformed: Math.floor(Math.random() * 500),
          lastLoginAt: new Date().toISOString(),
          totalSessionTime: Math.floor(Math.random() * 10000),
          favoriteFeatures: ['search', 'upload', 'ocr'],
        },
        permissions: getUserPermissions(authContext.user.role || 'user'),
        sessionInfo: sessionInfo || createSessionInfo(),
      }
      setUserProfile(enhancedUser)
    } else if (!authContext?.user) {
      setUserProfile(null)
      setSessionInfo(null)
    }
  }, [authContext?.user, userProfile, sessionInfo])

  const loginAs = useCallback(async (userType: UserType | MockUser) => {
    if (!authContext) return

    let user: MockUser

    if (typeof userType === 'string') {
      user = createUserByType(userType)
    } else {
      user = userType
    }

    const newSessionInfo = createSessionInfo()
    setSessionInfo(newSessionInfo)
    
    await authContext.login(user)
    
    recordActivity('login', { userType: typeof userType === 'string' ? userType : 'custom' })
  }, [authContext])

  const logout = useCallback(async () => {
    if (!authContext) return

    recordActivity('logout')
    await authContext.logout()
    setSessionInfo(null)
  }, [authContext])

  const switchUser = useCallback(async (userId: string) => {
    if (!authContext) return

    // In a real app, you'd fetch the user by ID
    const newUser = createMockUser({ id: userId })
    await loginAs(newUser)
  }, [authContext, loginAs])

  const updateProfile = useCallback((updates: Partial<UserProfile>) => {
    if (userProfile) {
      const updatedProfile = { ...userProfile, ...updates }
      setUserProfile(updatedProfile)
      
      if (authContext) {
        authContext.updateUser(updatedProfile)
      }
      
      recordActivity('profile_update', { fields: Object.keys(updates) })
    }
  }, [userProfile, authContext])

  const updatePreferences = useCallback((preferences: Partial<UserPreferences>) => {
    if (userProfile) {
      const updatedPreferences = { ...userProfile.preferences, ...preferences }
      updateProfile({ preferences: updatedPreferences })
      recordActivity('preferences_update', { preferences: Object.keys(preferences) })
    }
  }, [userProfile, updateProfile])

  const resetPreferences = useCallback(() => {
    updateProfile({ preferences: DEFAULT_PREFERENCES })
    recordActivity('preferences_reset')
  }, [updateProfile])

  // User scenario simulations
  const simulateNewUser = useCallback(async () => {
    const newUser = createMockUserWithScenario('new_user')
    await loginAs(newUser)
  }, [loginAs])

  const simulateExperiencedUser = useCallback(async () => {
    const experiencedUser = createMockUserWithScenario('typical_user')
    await loginAs({
      ...experiencedUser,
      created_at: new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 year ago
    })
  }, [loginAs])

  const simulateAdminUser = useCallback(async () => {
    const adminUser = createMockUserWithScenario('admin_user')
    await loginAs(adminUser)
  }, [loginAs])

  const simulateGuestUser = useCallback(async () => {
    const guestUser = createMockUser({
      username: 'guest',
      email: 'guest@example.com',
      role: 'guest',
      is_verified: false,
    })
    await loginAs(guestUser)
  }, [loginAs])

  // Authentication scenarios
  const simulateSessionExpiry = useCallback(() => {
    if (authContext) {
      authContext.simulateTokenExpiry()
      recordActivity('session_expired')
    }
  }, [authContext])

  const simulateAccountLock = useCallback(() => {
    if (authContext) {
      authContext.simulateAccountLock()
      recordActivity('account_locked')
    }
  }, [authContext])

  const simulatePasswordReset = useCallback(async () => {
    recordActivity('password_reset_requested')
    // Simulate password reset flow
    await new Promise(resolve => setTimeout(resolve, 1000))
    recordActivity('password_reset_completed')
  }, [])

  const simulateOIDCLogin = useCallback(async () => {
    const oidcUser = createMockUserWithScenario('oidc_user')
    await loginAs(oidcUser)
    recordActivity('oidc_login')
  }, [loginAs])

  // User behavior simulation
  const simulateUserActivity = useCallback((actions: UserAction[]) => {
    actions.forEach((action, index) => {
      setTimeout(() => {
        recordActivity(action.type, {
          target: action.target,
          ...action.metadata,
        })
        
        // Update statistics based on action
        if (userProfile) {
          const updatedStats = { ...userProfile.statistics }
          if (action.type === 'upload') {
            updatedStats.documentsUploaded = (updatedStats.documentsUploaded || 0) + 1
          } else if (action.type === 'search') {
            updatedStats.searchesPerformed = (updatedStats.searchesPerformed || 0) + 1
          }
          updateProfile({ statistics: updatedStats })
        }
      }, (action.delay || 1000) * index)
    })
  }, [userProfile, updateProfile])

  const simulateInactivity = useCallback((duration: number) => {
    recordActivity('inactivity_start', { duration })
    
    setTimeout(() => {
      recordActivity('inactivity_end')
      
      if (sessionInfo) {
        setSessionInfo(prev => prev ? {
          ...prev,
          lastActivity: new Date(Date.now() - duration),
          isActive: false,
        } : null)
      }
    }, duration)
  }, [sessionInfo])

  const simulateMultipleLogins = useCallback(() => {
    recordActivity('multiple_login_detected')
    // In a real app, this might force logout or show a warning
  }, [])

  // Activity tracking
  const recordActivity = useCallback((action: string, metadata?: any) => {
    const record: ActivityRecord = {
      timestamp: new Date(),
      action,
      metadata,
      sessionId: sessionInfo?.sessionId || 'no-session',
    }
    
    setActivityHistory(prev => [record, ...prev].slice(0, 100)) // Keep last 100 activities
    
    // Update session last activity
    if (sessionInfo) {
      setSessionInfo(prev => prev ? {
        ...prev,
        lastActivity: new Date(),
        isActive: true,
      } : null)
    }
  }, [sessionInfo])

  const getActivityHistory = useCallback((): ActivityRecord[] => {
    return activityHistory
  }, [activityHistory])

  const getUserInsights = useCallback((): UserInsights => {
    const actionCounts = activityHistory.reduce((acc, record) => {
      acc[record.action] = (acc[record.action] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    const mostUsedFeatures = Object.entries(actionCounts)
      .sort(([,a], [,b]) => b - a)
      .slice(0, 5)
      .map(([feature]) => feature)

    const sessionDurations = activityHistory
      .filter(r => r.action === 'logout')
      .map(r => r.metadata?.sessionDuration || 0)
    const averageSessionDuration = sessionDurations.length > 0
      ? sessionDurations.reduce((sum, dur) => sum + dur, 0) / sessionDurations.length
      : 0

    const totalActions = activityHistory.length
    const productivityScore = Math.min(100, Math.floor((totalActions / 10) * 10)) // Simplified scoring

    return {
      mostUsedFeatures,
      averageSessionDuration,
      productivityScore,
      usagePatterns: generateUsagePatterns(activityHistory),
      recommendations: generateRecommendations(actionCounts, userProfile),
    }
  }, [activityHistory, userProfile])

  const createTestUser = useCallback((overrides: Partial<MockUser> = {}): MockUser => {
    return createMockUser(overrides)
  }, [])

  const cloneCurrentUser = useCallback((): MockUser | null => {
    if (!userProfile) return null
    
    return {
      id: userProfile.id,
      username: userProfile.username,
      email: userProfile.email,
      role: userProfile.role,
      created_at: userProfile.created_at,
      updated_at: userProfile.updated_at,
      is_verified: userProfile.is_verified,
      settings: userProfile.settings,
    }
  }, [userProfile])

  const reset = useCallback(() => {
    setUserProfile(null)
    setActivityHistory([])
    setSessionInfo(null)
    authContext?.reset?.()
  }, [authContext])

  return {
    user: userProfile,
    isAuthenticated: authContext?.isAuthenticated || false,
    isLoading: authContext?.isLoading || false,
    loginAs,
    logout,
    switchUser,
    updateProfile,
    updatePreferences,
    resetPreferences,
    simulateNewUser,
    simulateExperiencedUser,
    simulateAdminUser,
    simulateGuestUser,
    simulateSessionExpiry,
    simulateAccountLock,
    simulatePasswordReset,
    simulateOIDCLogin,
    simulateUserActivity,
    simulateInactivity,
    simulateMultipleLogins,
    recordActivity,
    getActivityHistory,
    getUserInsights,
    createTestUser,
    cloneCurrentUser,
    reset,
  }
}

// Helper functions
function createUserByType(type: UserType): MockUser {
  switch (type) {
    case 'admin':
      return createMockUserWithScenario('admin_user')
    case 'new_user':
      return createMockUserWithScenario('new_user')
    case 'experienced_user':
      return createMockUserWithScenario('typical_user')
    case 'guest':
      return createMockUser({ role: 'guest', is_verified: false })
    case 'editor':
      return createMockUser({ role: 'editor' })
    case 'user':
    default:
      return createMockUserWithScenario('typical_user')
  }
}

function getUserPermissions(role: string): string[] {
  switch (role) {
    case 'admin':
      return ['read', 'write', 'delete', 'admin', 'manage_users', 'manage_settings']
    case 'editor':
      return ['read', 'write', 'delete']
    case 'guest':
      return ['read']
    case 'user':
    default:
      return ['read', 'write']
  }
}

function createSessionInfo(): SessionInfo {
  return {
    sessionId: `session_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    loginTime: new Date(),
    lastActivity: new Date(),
    ipAddress: `192.168.1.${Math.floor(Math.random() * 254) + 1}`,
    userAgent: 'Mock User Agent',
    isActive: true,
  }
}

function generateUsagePatterns(history: ActivityRecord[]): string[] {
  const patterns: string[] = []
  
  // Analyze time patterns
  const hours = history.map(r => r.timestamp.getHours())
  const mostActiveHour = hours.sort((a,b) => 
    hours.filter(h => h === a).length - hours.filter(h => h === b).length
  ).pop()
  
  if (mostActiveHour !== undefined) {
    if (mostActiveHour < 12) patterns.push('morning_user')
    else if (mostActiveHour < 18) patterns.push('afternoon_user')
    else patterns.push('evening_user')
  }
  
  // Analyze action patterns
  const actionTypes = history.map(r => r.action)
  if (actionTypes.filter(a => a === 'search').length > actionTypes.length * 0.4) {
    patterns.push('search_heavy')
  }
  if (actionTypes.filter(a => a === 'upload').length > actionTypes.length * 0.3) {
    patterns.push('upload_heavy')
  }
  
  return patterns
}

function generateRecommendations(actionCounts: Record<string, number>, profile: UserProfile | null): string[] {
  const recommendations: string[] = []
  
  if (actionCounts.search > 10) {
    recommendations.push('Consider using saved searches for frequently used queries')
  }
  if (actionCounts.upload > 5 && !actionCounts.delete) {
    recommendations.push('Regular cleanup of old documents can improve performance')
  }
  if (profile?.preferences?.dashboard.layout === 'list') {
    recommendations.push('Try grid view for better document previews')
  }
  
  return recommendations
}