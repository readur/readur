import React, { createContext, useContext, useEffect, useState } from 'react'
import { api } from '../services/api'
import type { AuthConfig } from '../types/generated'

interface FeatureFlags {
  allowLocalAuth: boolean
  /** Self-registration is open (only ever true together with local sign-in). */
  allowRegistration: boolean
  oidcEnabled: boolean
  enablePerUserWatch: boolean
}

interface FeatureFlagsContextType {
  flags: FeatureFlags
  loading: boolean
  error: Error | null
}

const defaultFlags: FeatureFlags = {
  allowLocalAuth: true,
  allowRegistration: false,
  oidcEnabled: false,
  enablePerUserWatch: false,
}

export const FeatureFlagsContext = createContext<FeatureFlagsContextType | undefined>(undefined)

export function FeatureFlagsProvider({ children }: { children: React.ReactNode }) {
  const [flags, setFlags] = useState<FeatureFlags>(defaultFlags)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)

  useEffect(() => {
    const fetchConfig = async () => {
      try {
        const response = await api.get<AuthConfig>('/auth/config')

        setFlags({
          allowLocalAuth: response.data.allow_local_auth,
          allowRegistration: response.data.allow_local_auth && response.data.allow_registration === true,
          oidcEnabled: response.data.oidc_enabled,
          enablePerUserWatch: response.data.enable_per_user_watch,
        })
      } catch (err) {
        setError(err instanceof Error ? err : new Error('Failed to fetch feature flags'))
        setFlags({
          ...defaultFlags,
          enablePerUserWatch: false,
        })
      } finally {
        setLoading(false)
      }
    }

    fetchConfig()
  }, [])

  const value = {
    flags,
    loading,
    error,
  }

  return <FeatureFlagsContext.Provider value={value}>{children}</FeatureFlagsContext.Provider>
}

export function useFeatureFlags() {
  const context = useContext(FeatureFlagsContext)
  if (context === undefined) {
    throw new Error('useFeatureFlags must be used within a FeatureFlagsProvider')
  }
  return context
}
