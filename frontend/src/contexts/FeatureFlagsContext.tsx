import React, { createContext, useContext, useEffect, useState } from 'react'
import { api } from '../services/api'

interface FeatureFlags {
  allowLocalAuth: boolean
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
        const response = await api.get<{
          allow_local_auth: boolean
          oidc_enabled: boolean
          enable_per_user_watch: boolean
        }>('/auth/config')

        setFlags({
          allowLocalAuth: response.data.allow_local_auth,
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
