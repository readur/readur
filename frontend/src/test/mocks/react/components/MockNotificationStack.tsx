/**
 * MockNotificationStack - Notification testing component
 * Provides comprehensive notification display and interaction testing
 */

import React, { useState, useCallback, useEffect } from 'react'
import { useMockNotificationContext } from '../providers/MockNotificationProvider'
import type { MockNotification } from '../../api/types'

export interface MockNotificationStackProps {
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center'
  maxVisible?: number
  autoHide?: boolean
  autoHideDelay?: number
  enableSounds?: boolean
  enableGrouping?: boolean
  enableActions?: boolean
  showProgress?: boolean
  animationDuration?: number
  className?: string
  style?: React.CSSProperties
}

interface NotificationGroup {
  type: string
  count: number
  latestNotification: MockNotification
  notifications: MockNotification[]
}

export const MockNotificationStack: React.FC<MockNotificationStackProps> = ({
  position = 'top-right',
  maxVisible = 5,
  autoHide = true,
  autoHideDelay = 5000,
  enableSounds = false,
  enableGrouping = true,
  enableActions = true,
  showProgress = true,
  animationDuration = 300,
  className = '',
  style = {},
}) => {
  const [groupedNotifications, setGroupedNotifications] = useState<NotificationGroup[]>([])
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set())
  const [dismissingIds, setDismissingIds] = useState<Set<string>>(new Set())

  const {
    notifications,
    removeNotification,
    markAsRead,
    clearNotifications,
  } = useMockNotificationContext()

  // Group notifications by type if enabled
  useEffect(() => {
    if (!enableGrouping) {
      const individual = notifications.slice(0, maxVisible).map(notification => ({
        type: notification.type,
        count: 1,
        latestNotification: notification,
        notifications: [notification],
      }))
      setGroupedNotifications(individual)
      return
    }

    const groups = new Map<string, NotificationGroup>()
    
    notifications.slice(0, maxVisible * 2).forEach(notification => {
      const key = notification.type
      if (groups.has(key)) {
        const group = groups.get(key)!
        group.count++
        group.notifications.push(notification)
        // Keep the latest notification as the representative
        if (new Date(notification.created_at) > new Date(group.latestNotification.created_at)) {
          group.latestNotification = notification
        }
      } else {
        groups.set(key, {
          type: notification.type,
          count: 1,
          latestNotification: notification,
          notifications: [notification],
        })
      }
    })

    const sortedGroups = Array.from(groups.values())
      .sort((a, b) => 
        new Date(b.latestNotification.created_at).getTime() - 
        new Date(a.latestNotification.created_at).getTime()
      )
      .slice(0, maxVisible)

    setGroupedNotifications(sortedGroups)
  }, [notifications, enableGrouping, maxVisible])

  // Auto-hide notifications
  useEffect(() => {
    if (!autoHide) return

    const timeouts: NodeJS.Timeout[] = []

    groupedNotifications.forEach(group => {
      if (!group.latestNotification.persistent) {
        const timeout = setTimeout(() => {
          handleDismiss(group.latestNotification.id)
        }, autoHideDelay)
        timeouts.push(timeout)
      }
    })

    return () => {
      timeouts.forEach(timeout => clearTimeout(timeout))
    }
  }, [groupedNotifications, autoHide, autoHideDelay])

  // Play notification sound
  useEffect(() => {
    if (enableSounds && groupedNotifications.length > 0) {
      const latestGroup = groupedNotifications[0]
      if (latestGroup.latestNotification.type === 'error') {
        // In a real app, you'd play actual sounds
        console.log('🔊 Error notification sound')
      }
    }
  }, [groupedNotifications, enableSounds])

  const handleDismiss = useCallback(async (notificationId: string) => {
    setDismissingIds(prev => new Set([...prev, notificationId]))
    
    // Wait for animation
    await new Promise(resolve => setTimeout(resolve, animationDuration))
    
    removeNotification(notificationId)
    setDismissingIds(prev => {
      const newSet = new Set(prev)
      newSet.delete(notificationId)
      return newSet
    })
  }, [removeNotification, animationDuration])

  const handleMarkAsRead = useCallback((notificationId: string) => {
    markAsRead(notificationId)
  }, [markAsRead])

  const handleGroupToggle = useCallback((groupType: string) => {
    setExpandedGroups(prev => {
      const newSet = new Set(prev)
      if (newSet.has(groupType)) {
        newSet.delete(groupType)
      } else {
        newSet.add(groupType)
      }
      return newSet
    })
  }, [])

  const handleClearAll = useCallback(() => {
    clearNotifications()
  }, [clearNotifications])

  const getPositionStyles = (): React.CSSProperties => {
    const baseStyles: React.CSSProperties = {
      position: 'fixed',
      zIndex: 9999,
      pointerEvents: 'none',
      display: 'flex',
      flexDirection: 'column',
      gap: '0.75rem',
      maxWidth: '400px',
      width: '100%',
      ...style,
    }

    switch (position) {
      case 'top-right':
        return { ...baseStyles, top: '1rem', right: '1rem' }
      case 'top-left':
        return { ...baseStyles, top: '1rem', left: '1rem' }
      case 'bottom-right':
        return { ...baseStyles, bottom: '1rem', right: '1rem', flexDirection: 'column-reverse' }
      case 'bottom-left':
        return { ...baseStyles, bottom: '1rem', left: '1rem', flexDirection: 'column-reverse' }
      case 'top-center':
        return { ...baseStyles, top: '1rem', left: '50%', transform: 'translateX(-50%)' }
      case 'bottom-center':
        return { ...baseStyles, bottom: '1rem', left: '50%', transform: 'translateX(-50%)', flexDirection: 'column-reverse' }
      default:
        return { ...baseStyles, top: '1rem', right: '1rem' }
    }
  }

  const getNotificationIcon = (type: string): string => {
    switch (type) {
      case 'success': return '✅'
      case 'error': return '❌'
      case 'warning': return '⚠️'
      case 'info': return 'ℹ️'
      default: return '📋'
    }
  }

  const getNotificationColor = (type: string): { bg: string; border: string; text: string } => {
    switch (type) {
      case 'success':
        return { bg: '#f0fdf4', border: '#bbf7d0', text: '#166534' }
      case 'error':
        return { bg: '#fef2f2', border: '#fecaca', text: '#dc2626' }
      case 'warning':
        return { bg: '#fffbeb', border: '#fed7aa', text: '#d97706' }
      case 'info':
      default:
        return { bg: '#eff6ff', border: '#bfdbfe', text: '#2563eb' }
    }
  }

  if (groupedNotifications.length === 0) {
    return null
  }

  return (
    <>
      <style>
        {`
          @keyframes slideIn {
            from {
              opacity: 0;
              transform: translateX(${position.includes('right') ? '100%' : position.includes('left') ? '-100%' : '0'}) translateY(${position.includes('top') ? '-20px' : '20px'});
            }
            to {
              opacity: 1;
              transform: translateX(0) translateY(0);
            }
          }
          
          @keyframes slideOut {
            from {
              opacity: 1;
              transform: translateX(0) translateY(0);
            }
            to {
              opacity: 0;
              transform: translateX(${position.includes('right') ? '100%' : position.includes('left') ? '-100%' : '0'}) translateY(${position.includes('top') ? '-20px' : '20px'});
            }
          }
          
          @keyframes progress {
            from { width: 100%; }
            to { width: 0%; }
          }
        `}
      </style>

      <div
        style={getPositionStyles()}
        className={className}
        data-testid="notification-stack"
      >
        {/* Clear all button */}
        {groupedNotifications.length > 1 && (
          <div style={{ pointerEvents: 'auto', textAlign: 'right', marginBottom: '0.5rem' }}>
            <button
              onClick={handleClearAll}
              style={{
                background: '#6b7280',
                color: 'white',
                border: 'none',
                padding: '0.25rem 0.5rem',
                borderRadius: '4px',
                fontSize: '0.75rem',
                cursor: 'pointer',
                opacity: 0.8,
              }}
              data-testid="clear-all-notifications"
            >
              Clear All ({groupedNotifications.reduce((sum, group) => sum + group.count, 0)})
            </button>
          </div>
        )}

        {groupedNotifications.map((group) => (
          <NotificationGroupItem
            key={`${group.type}-${group.latestNotification.id}`}
            group={group}
            isExpanded={expandedGroups.has(group.type)}
            isDismissing={dismissingIds.has(group.latestNotification.id)}
            onToggle={() => handleGroupToggle(group.type)}
            onDismiss={handleDismiss}
            onMarkAsRead={handleMarkAsRead}
            enableGrouping={enableGrouping}
            enableActions={enableActions}
            showProgress={showProgress && autoHide}
            autoHideDelay={autoHideDelay}
            animationDuration={animationDuration}
            getNotificationIcon={getNotificationIcon}
            getNotificationColor={getNotificationColor}
          />
        ))}
      </div>
    </>
  )
}

// Individual notification group component
const NotificationGroupItem: React.FC<{
  group: NotificationGroup
  isExpanded: boolean
  isDismissing: boolean
  onToggle: () => void
  onDismiss: (id: string) => void
  onMarkAsRead: (id: string) => void
  enableGrouping: boolean
  enableActions: boolean
  showProgress: boolean
  autoHideDelay: number
  animationDuration: number
  getNotificationIcon: (type: string) => string
  getNotificationColor: (type: string) => { bg: string; border: string; text: string }
}> = ({
  group,
  isExpanded,
  isDismissing,
  onToggle,
  onDismiss,
  onMarkAsRead,
  enableGrouping,
  enableActions,
  showProgress,
  autoHideDelay,
  animationDuration,
  getNotificationIcon,
  getNotificationColor,
}) => {
  const [progressStartTime] = useState(Date.now())
  const colors = getNotificationColor(group.type)
  const notification = group.latestNotification

  const formatTimeAgo = (dateString: string): string => {
    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffSeconds = Math.floor(diffMs / 1000)
    const diffMinutes = Math.floor(diffSeconds / 60)
    const diffHours = Math.floor(diffMinutes / 60)

    if (diffSeconds < 60) return 'just now'
    if (diffMinutes < 60) return `${diffMinutes}m ago`
    if (diffHours < 24) return `${diffHours}h ago`
    return date.toLocaleDateString()
  }

  return (
    <div
      style={{
        pointerEvents: 'auto',
        animation: isDismissing 
          ? `slideOut ${animationDuration}ms ease-out forwards`
          : `slideIn ${animationDuration}ms ease-out`,
      }}
      data-testid={`notification-group-${group.type}`}
    >
      {/* Main notification */}
      <div
        style={{
          background: colors.bg,
          border: `1px solid ${colors.border}`,
          borderRadius: '12px',
          padding: '1rem',
          boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
          position: 'relative',
          overflow: 'hidden',
          fontFamily: 'Inter, system-ui, sans-serif',
        }}
      >
        {/* Progress bar for auto-hide */}
        {showProgress && !notification.persistent && (
          <div
            style={{
              position: 'absolute',
              bottom: 0,
              left: 0,
              height: '3px',
              backgroundColor: colors.text,
              opacity: 0.3,
              animation: `progress ${autoHideDelay}ms linear`,
            }}
          />
        )}

        <div style={{ display: 'flex', alignItems: 'flex-start', gap: '0.75rem' }}>
          {/* Icon */}
          <div
            style={{
              fontSize: '1.25rem',
              flexShrink: 0,
              marginTop: '0.125rem',
            }}
          >
            {getNotificationIcon(group.type)}
          </div>

          {/* Content */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'flex-start',
              marginBottom: '0.5rem',
            }}>
              <div style={{ flex: 1 }}>
                <p style={{
                  margin: 0,
                  fontSize: '0.875rem',
                  fontWeight: '500',
                  color: colors.text,
                  lineHeight: '1.4',
                }}>
                  {notification.message}
                  {enableGrouping && group.count > 1 && (
                    <span
                      style={{
                        marginLeft: '0.5rem',
                        background: colors.text,
                        color: colors.bg,
                        padding: '0.125rem 0.375rem',
                        borderRadius: '12px',
                        fontSize: '0.75rem',
                        fontWeight: '600',
                      }}
                    >
                      {group.count}
                    </span>
                  )}
                </p>
                
                <p style={{
                  margin: '0.25rem 0 0 0',
                  fontSize: '0.75rem',
                  color: colors.text,
                  opacity: 0.7,
                }}>
                  {formatTimeAgo(notification.created_at)}
                </p>
              </div>

              {/* Actions */}
              {enableActions && (
                <div style={{ display: 'flex', gap: '0.25rem', flexShrink: 0 }}>
                  {!notification.read && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        onMarkAsRead(notification.id)
                      }}
                      style={{
                        background: 'rgba(255, 255, 255, 0.8)',
                        border: 'none',
                        borderRadius: '4px',
                        padding: '0.25rem',
                        cursor: 'pointer',
                        opacity: 0.8,
                        fontSize: '0.75rem',
                      }}
                      title="Mark as read"
                    >
                      ✓
                    </button>
                  )}
                  
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      onDismiss(notification.id)
                    }}
                    style={{
                      background: 'rgba(255, 255, 255, 0.8)',
                      border: 'none',
                      borderRadius: '4px',
                      padding: '0.25rem',
                      cursor: 'pointer',
                      opacity: 0.8,
                      fontSize: '0.75rem',
                    }}
                    title="Dismiss"
                  >
                    ×
                  </button>
                </div>
              )}
            </div>

            {/* Group expansion toggle */}
            {enableGrouping && group.count > 1 && (
              <button
                onClick={onToggle}
                style={{
                  background: 'transparent',
                  border: `1px solid ${colors.text}`,
                  color: colors.text,
                  borderRadius: '6px',
                  padding: '0.25rem 0.5rem',
                  fontSize: '0.75rem',
                  cursor: 'pointer',
                  opacity: 0.8,
                  marginTop: '0.5rem',
                }}
              >
                {isExpanded ? 'Show less' : `Show all ${group.count} notifications`}
              </button>
            )}

            {/* Notification actions */}
            {notification.actions && notification.actions.length > 0 && (
              <div style={{
                display: 'flex',
                gap: '0.5rem',
                marginTop: '0.75rem',
              }}>
                {notification.actions.map((action, index) => (
                  <button
                    key={index}
                    onClick={(e) => {
                      e.stopPropagation()
                      action.action()
                    }}
                    style={{
                      background: action.style === 'primary' ? colors.text : 'transparent',
                      color: action.style === 'primary' ? colors.bg : colors.text,
                      border: `1px solid ${colors.text}`,
                      borderRadius: '6px',
                      padding: '0.375rem 0.75rem',
                      fontSize: '0.75rem',
                      fontWeight: '500',
                      cursor: 'pointer',
                    }}
                  >
                    {action.label}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Expanded notifications */}
      {isExpanded && group.notifications.slice(1).map((expandedNotification) => (
        <div
          key={expandedNotification.id}
          style={{
            background: colors.bg,
            border: `1px solid ${colors.border}`,
            borderRadius: '8px',
            padding: '0.75rem',
            marginTop: '0.5rem',
            marginLeft: '1rem',
            opacity: 0.9,
            fontSize: '0.875rem',
          }}
          data-testid={`expanded-notification-${expandedNotification.id}`}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ color: colors.text }}>{expandedNotification.message}</span>
            <div style={{ display: 'flex', gap: '0.25rem' }}>
              <span style={{ fontSize: '0.75rem', color: colors.text, opacity: 0.7 }}>
                {formatTimeAgo(expandedNotification.created_at)}
              </span>
              {enableActions && (
                <button
                  onClick={() => onDismiss(expandedNotification.id)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    color: colors.text,
                    cursor: 'pointer',
                    fontSize: '0.75rem',
                    opacity: 0.7,
                  }}
                >
                  ×
                </button>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  )
}