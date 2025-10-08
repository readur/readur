/**
 * React Providers - Mock providers for comprehensive testing
 * Centralized exports for all mock provider components
 */

export { 
  MockApiProvider, 
  useMockApiContext, 
  withMockApi,
  type MockApiContextType,
  type MockApiProviderProps 
} from './MockApiProvider'

export { 
  MockAuthProvider, 
  useMockAuthContext, 
  withMockAuth,
  type MockAuthContextType,
  type MockAuthProviderProps,
  type LoginOptions,
  type AuthState 
} from './MockAuthProvider'

export { 
  MockNotificationProvider, 
  useMockNotificationContext, 
  withMockNotifications,
  type MockNotificationContextType,
  type MockNotificationProviderProps,
  type NotificationOptions,
  type NotificationAction,
  type NotificationStats 
} from './MockNotificationProvider'

export { 
  MockWebSocketProvider, 
  useMockWebSocketContext, 
  withMockWebSocket,
  type MockWebSocketContextType,
  type MockWebSocketProviderProps,
  type ConnectionStats 
} from './MockWebSocketProvider'

// Combined provider for comprehensive testing setup
import React, { ReactNode } from 'react'
import { MockApiProvider, MockApiProviderProps } from './MockApiProvider'
import { MockAuthProvider, MockAuthProviderProps } from './MockAuthProvider'
import { MockNotificationProvider, MockNotificationProviderProps } from './MockNotificationProvider'
import { MockWebSocketProvider, MockWebSocketProviderProps } from './MockWebSocketProvider'

export interface ComprehensiveMockProviderProps {
  children: ReactNode
  api?: Partial<MockApiProviderProps>
  auth?: Partial<MockAuthProviderProps>
  notifications?: Partial<MockNotificationProviderProps>
  websocket?: Partial<MockWebSocketProviderProps>
  enableAll?: boolean
}

/**
 * ComprehensiveMockProvider - All-in-one provider for complete testing setup
 * Combines all mock providers for easy testing of complex scenarios
 */
export const ComprehensiveMockProvider: React.FC<ComprehensiveMockProviderProps> = ({
  children,
  api = {},
  auth = {},
  notifications = {},
  websocket = {},
  enableAll = true,
}) => {
  let content = children

  // Wrap with WebSocket provider (innermost)
  if (enableAll || websocket) {
    content = (
      <MockWebSocketProvider {...websocket}>
        {content}
      </MockWebSocketProvider>
    )
  }

  // Wrap with Notification provider
  if (enableAll || notifications) {
    content = (
      <MockNotificationProvider {...notifications}>
        {content}
      </MockNotificationProvider>
    )
  }

  // Wrap with Auth provider
  if (enableAll || auth) {
    content = (
      <MockAuthProvider {...auth}>
        {content}
      </MockAuthProvider>
    )
  }

  // Wrap with API provider (outermost)
  if (enableAll || api) {
    content = (
      <MockApiProvider {...api}>
        {content}
      </MockApiProvider>
    )
  }

  return <>{content}</>
}

// Convenience HOC for comprehensive setup
export const withComprehensiveMocks = <P extends object>(
  Component: React.ComponentType<P>,
  providerProps?: Partial<ComprehensiveMockProviderProps>
) => {
  const WrappedComponent = (props: P) => (
    <ComprehensiveMockProvider {...providerProps}>
      <Component {...props} />
    </ComprehensiveMockProvider>
  )
  
  WrappedComponent.displayName = `withComprehensiveMocks(${Component.displayName || Component.name})`
  return WrappedComponent
}