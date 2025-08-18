/**
 * MockNotificationProvider - Notification system testing
 * Provides comprehensive notification state management and testing utilities
 */

import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react'
import { createMockNotification } from '../../factories/combined'
import type { MockNotification } from '../../api/types'

export interface NotificationOptions {
  type?: 'info' | 'success' | 'warning' | 'error'
  duration?: number
  persistent?: boolean
  actions?: NotificationAction[]
  data?: any
}

export interface NotificationAction {
  label: string
  action: () => void
  style?: 'primary' | 'secondary' | 'danger'
}

export interface MockNotificationContextType {
  // Notification state
  notifications: MockNotification[]
  unreadCount: number
  
  // Notification actions
  addNotification: (message: string, options?: NotificationOptions) => string
  removeNotification: (id: string) => void
  clearNotifications: () => void
  markAsRead: (id: string) => void
  markAllAsRead: () => void
  
  // Bulk operations
  removeByType: (type: string) => void
  getNotificationsByType: (type: string) => MockNotification[]
  
  // Testing utilities
  simulateSystemNotification: (type?: 'upload_complete' | 'ocr_failed' | 'sync_progress' | 'error') => void
  simulateBulkNotifications: (count?: number, type?: string) => void
  simulateNotificationFlow: (scenario?: 'document_processing' | 'sync_workflow' | 'error_cascade') => void
  
  // Configuration
  setGlobalDuration: (duration: number) => void
  enableSounds: (enabled: boolean) => void
  setMaxNotifications: (max: number) => void
  
  // State utilities
  reset: () => void
  getNotificationStats: () => NotificationStats
}

export interface NotificationStats {
  total: number
  unread: number
  byType: Record<string, number>
  oldestUnread: MockNotification | null
  newestNotification: MockNotification | null
}

const MockNotificationContext = createContext<MockNotificationContextType | null>(null)

export interface MockNotificationProviderProps {
  children: ReactNode
  maxNotifications?: number
  defaultDuration?: number
  enableAutoRemove?: boolean
  enableSounds?: boolean
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center'
}

export const MockNotificationProvider: React.FC<MockNotificationProviderProps> = ({
  children,
  maxNotifications = 10,
  defaultDuration = 5000,
  enableAutoRemove = true,
  enableSounds = false,
  position = 'top-right',
}) => {
  const [notifications, setNotifications] = useState<MockNotification[]>([])
  const [globalDuration, setGlobalDuration] = useState(defaultDuration)
  const [soundsEnabled, setSoundsEnabled] = useState(enableSounds)
  const [maxNotifs, setMaxNotifs] = useState(maxNotifications)

  // Auto-remove expired notifications
  useEffect(() => {
    if (!enableAutoRemove) return

    const interval = setInterval(() => {
      setNotifications(prev => prev.filter(notification => {
        if (notification.persistent) return true
        const now = Date.now()
        const createdAt = new Date(notification.created_at).getTime()
        return (now - createdAt) < (notification.duration || globalDuration)
      }))
    }, 1000)

    return () => clearInterval(interval)
  }, [enableAutoRemove, globalDuration])

  const addNotification = useCallback((message: string, options: NotificationOptions = {}) => {
    const notification = createMockNotification({
      message,
      type: options.type || 'info',
      duration: options.duration || globalDuration,
      persistent: options.persistent || false,
      actions: options.actions || [],
      data: options.data,
      read: false,
    })

    setNotifications(prev => {
      const newNotifications = [notification, ...prev]
      
      // Limit max notifications
      if (newNotifications.length > maxNotifs) {
        return newNotifications.slice(0, maxNotifs)
      }
      
      return newNotifications
    })

    // Play sound if enabled
    if (soundsEnabled && options.type === 'error') {
      // In a real implementation, you'd play an actual sound
      console.log('🔊 Error notification sound')
    }

    return notification.id
  }, [globalDuration, maxNotifs, soundsEnabled])

  const removeNotification = useCallback((id: string) => {
    setNotifications(prev => prev.filter(n => n.id !== id))
  }, [])

  const clearNotifications = useCallback(() => {
    setNotifications([])
  }, [])

  const markAsRead = useCallback((id: string) => {
    setNotifications(prev => prev.map(n => 
      n.id === id ? { ...n, read: true, read_at: new Date().toISOString() } : n
    ))
  }, [])

  const markAllAsRead = useCallback(() => {
    const now = new Date().toISOString()
    setNotifications(prev => prev.map(n => ({ 
      ...n, 
      read: true, 
      read_at: n.read_at || now 
    })))
  }, [])

  const removeByType = useCallback((type: string) => {
    setNotifications(prev => prev.filter(n => n.type !== type))
  }, [])

  const getNotificationsByType = useCallback((type: string) => {
    return notifications.filter(n => n.type === type)
  }, [notifications])

  const simulateSystemNotification = useCallback((type: 'upload_complete' | 'ocr_failed' | 'sync_progress' | 'error' = 'info') => {
    const messages = {
      upload_complete: 'Document uploaded and processed successfully',
      ocr_failed: 'OCR processing failed for document.pdf',
      sync_progress: 'Sync in progress: 15 documents remaining',
      error: 'An unexpected error occurred',
    }

    const notificationTypes = {
      upload_complete: 'success' as const,
      ocr_failed: 'error' as const,
      sync_progress: 'info' as const,
      error: 'error' as const,
    }

    addNotification(messages[type], {
      type: notificationTypes[type],
      duration: type === 'error' ? 0 : globalDuration, // Errors persist
      persistent: type === 'error',
    })
  }, [addNotification, globalDuration])

  const simulateBulkNotifications = useCallback((count: number = 5, type: string = 'info') => {
    for (let i = 0; i < count; i++) {
      setTimeout(() => {
        addNotification(`Bulk notification ${i + 1}`, { 
          type: type as any,
          duration: globalDuration + (i * 1000), // Stagger removal
        })
      }, i * 200) // Stagger creation
    }
  }, [addNotification, globalDuration])

  const simulateNotificationFlow = useCallback((scenario: 'document_processing' | 'sync_workflow' | 'error_cascade' = 'document_processing') => {
    switch (scenario) {
      case 'document_processing':
        addNotification('Document upload started', { type: 'info' })
        setTimeout(() => addNotification('OCR processing...', { type: 'info' }), 1000)
        setTimeout(() => addNotification('Document processed successfully', { type: 'success' }), 3000)
        break
        
      case 'sync_workflow':
        addNotification('Starting sync...', { type: 'info' })
        setTimeout(() => addNotification('Discovering files...', { type: 'info' }), 500)
        setTimeout(() => addNotification('Processing 25 files...', { type: 'info' }), 1500)
        setTimeout(() => addNotification('Sync completed', { type: 'success' }), 4000)
        break
        
      case 'error_cascade':
        addNotification('Connection warning', { type: 'warning' })
        setTimeout(() => addNotification('Retry attempt failed', { type: 'error', persistent: true }), 1000)
        setTimeout(() => addNotification('Service temporarily unavailable', { type: 'error', persistent: true }), 2000)
        break
    }
  }, [addNotification])

  const reset = useCallback(() => {
    setNotifications([])
    setGlobalDuration(defaultDuration)
    setSoundsEnabled(enableSounds)
    setMaxNotifs(maxNotifications)
  }, [defaultDuration, enableSounds, maxNotifications])

  const getNotificationStats = useCallback((): NotificationStats => {
    const unreadNotifications = notifications.filter(n => !n.read)
    const typeStats = notifications.reduce((acc, n) => {
      acc[n.type] = (acc[n.type] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    return {
      total: notifications.length,
      unread: unreadNotifications.length,
      byType: typeStats,
      oldestUnread: unreadNotifications.sort((a, b) => 
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      )[0] || null,
      newestNotification: notifications[0] || null,
    }
  }, [notifications])

  const unreadCount = notifications.filter(n => !n.read).length

  const contextValue: MockNotificationContextType = {
    notifications,
    unreadCount,
    addNotification,
    removeNotification,
    clearNotifications,
    markAsRead,
    markAllAsRead,
    removeByType,
    getNotificationsByType,
    simulateSystemNotification,
    simulateBulkNotifications,
    simulateNotificationFlow,
    setGlobalDuration,
    enableSounds: setSoundsEnabled,
    setMaxNotifications: setMaxNotifs,
    reset,
    getNotificationStats,
  }

  return (
    <MockNotificationContext.Provider value={contextValue}>
      {children}
      <NotificationDisplay 
        notifications={notifications}
        position={position}
        onRemove={removeNotification}
        onMarkRead={markAsRead}
      />
    </MockNotificationContext.Provider>
  )
}

// Simple notification display component for testing
const NotificationDisplay: React.FC<{
  notifications: MockNotification[]
  position: string
  onRemove: (id: string) => void
  onMarkRead: (id: string) => void
}> = ({ notifications, position, onRemove, onMarkRead }) => {
  const getPositionStyles = () => {
    const baseStyles = {
      position: 'fixed' as const,
      zIndex: 9999,
      pointerEvents: 'none' as const,
      maxWidth: '400px',
    }

    switch (position) {
      case 'top-right':
        return { ...baseStyles, top: '16px', right: '16px' }
      case 'top-left':
        return { ...baseStyles, top: '16px', left: '16px' }
      case 'bottom-right':
        return { ...baseStyles, bottom: '16px', right: '16px' }
      case 'bottom-left':
        return { ...baseStyles, bottom: '16px', left: '16px' }
      case 'top-center':
        return { ...baseStyles, top: '16px', left: '50%', transform: 'translateX(-50%)' }
      case 'bottom-center':
        return { ...baseStyles, bottom: '16px', left: '50%', transform: 'translateX(-50%)' }
      default:
        return { ...baseStyles, top: '16px', right: '16px' }
    }
  }

  return (
    <div style={getPositionStyles()} data-testid="notification-container">
      {notifications.slice(0, 5).map((notification) => (
        <div
          key={notification.id}
          data-testid={`notification-${notification.type}`}
          style={{
            background: getNotificationColor(notification.type),
            color: 'white',
            padding: '12px 16px',
            marginBottom: '8px',
            borderRadius: '8px',
            boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
            pointerEvents: 'auto',
            opacity: notification.read ? 0.7 : 1,
            transition: 'opacity 0.2s ease',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
            <div style={{ flex: 1, marginRight: '8px' }}>
              {notification.message}
            </div>
            <div style={{ display: 'flex', gap: '4px' }}>
              {!notification.read && (
                <button
                  onClick={() => onMarkRead(notification.id)}
                  style={{ 
                    background: 'rgba(255,255,255,0.2)', 
                    border: 'none', 
                    color: 'white', 
                    padding: '2px 6px', 
                    borderRadius: '4px',
                    fontSize: '12px',
                    cursor: 'pointer'
                  }}
                >
                  ✓
                </button>
              )}
              <button
                onClick={() => onRemove(notification.id)}
                style={{ 
                  background: 'rgba(255,255,255,0.2)', 
                  border: 'none', 
                  color: 'white', 
                  padding: '2px 6px', 
                  borderRadius: '4px',
                  fontSize: '12px',
                  cursor: 'pointer'
                }}
              >
                ×
              </button>
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}

function getNotificationColor(type: string): string {
  switch (type) {
    case 'success': return '#10b981'
    case 'warning': return '#f59e0b'
    case 'error': return '#ef4444'
    case 'info':
    default: return '#3b82f6'
  }
}

export const useMockNotificationContext = (): MockNotificationContextType => {
  const context = useContext(MockNotificationContext)
  if (!context) {
    throw new Error('useMockNotificationContext must be used within a MockNotificationProvider')
  }
  return context
}

// High-order component for wrapping components with mock notifications
export const withMockNotifications = <P extends object>(
  Component: React.ComponentType<P>,
  providerProps?: Partial<MockNotificationProviderProps>
) => {
  const WrappedComponent = (props: P) => (
    <MockNotificationProvider {...providerProps}>
      <Component {...props} />
    </MockNotificationProvider>
  )
  
  WrappedComponent.displayName = `withMockNotifications(${Component.displayName || Component.name})`
  return WrappedComponent
}