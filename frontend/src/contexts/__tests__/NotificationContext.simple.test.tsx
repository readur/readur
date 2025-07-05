import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, act, render } from '@testing-library/react';
import { NotificationProvider, useNotifications } from '../NotificationContext';
import { renderWithProviders } from '../../test/test-utils';
import React from 'react';

// Simple test component
const SimpleTestComponent: React.FC = () => {
  const {
    notifications,
    unreadCount,
    addNotification,
    markAsRead,
    clearNotification,
  } = useNotifications();

  return (
    <div>
      <div data-testid="unread-count">{unreadCount}</div>
      <div data-testid="notifications-count">{notifications.length}</div>
      <button
        data-testid="add-notification"
        onClick={() =>
          addNotification({
            type: 'success',
            title: 'Test',
            message: 'Test message',
          })
        }
      >
        Add Notification
      </button>
      {notifications.map((notification) => (
        <div key={notification.id} data-testid={`notification-${notification.id}`}>
          <span data-testid={`title-${notification.id}`}>{notification.title}</span>
          <span data-testid={`read-${notification.id}`}>{notification.read.toString()}</span>
          <button
            data-testid={`mark-read-${notification.id}`}
            onClick={() => markAsRead(notification.id)}
          >
            Mark Read
          </button>
          <button
            data-testid={`clear-${notification.id}`}
            onClick={() => clearNotification(notification.id)}
          >
            Clear
          </button>
        </div>
      ))}
    </div>
  );
};

const renderWithProvider = () => {
  return renderWithProviders(
    <NotificationProvider>
      <SimpleTestComponent />
    </NotificationProvider>
  );
};

describe('NotificationContext - Simple Tests', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  test('should initialize with empty state', () => {
    renderWithProvider();
    
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
  });

  test('should add a notification and update counts', () => {
    renderWithProvider();
    
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');
  });

  test('should display notification content correctly', () => {
    renderWithProvider();
    
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    // Find the notification by searching for the title
    const titleElement = screen.getByText('Test');
    expect(titleElement).toBeInTheDocument();
    
    // Check if it's unread
    const readElement = screen.getByText('false');
    expect(readElement).toBeInTheDocument();
  });

  test('should mark notification as read', () => {
    renderWithProvider();
    
    // Add notification
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');

    // Mark as read
    act(() => {
      const markReadButton = screen.getByRole('button', { name: /mark read/i });
      markReadButton.click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    expect(screen.getByText('true')).toBeInTheDocument(); // Should be marked as read
  });

  test('should clear notification', () => {
    renderWithProvider();
    
    // Add notification
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');

    // Clear notification
    act(() => {
      const clearButton = screen.getByRole('button', { name: /clear/i });
      clearButton.click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
  });

  test('should handle multiple notifications', () => {
    renderWithProvider();
    
    // Add multiple notifications
    act(() => {
      screen.getByTestId('add-notification').click();
      screen.getByTestId('add-notification').click();
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('3');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('3');
  });

  test('should throw error when used outside provider', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    expect(() => {
      render(<SimpleTestComponent />);
    }).toThrow('useNotifications must be used within NotificationProvider');
    
    consoleSpy.mockRestore();
  });
});

// Test different notification types
describe('NotificationContext - Types', () => {
  const TypeTestComponent: React.FC<{ type: 'success' | 'error' | 'info' | 'warning' }> = ({ type }) => {
    const { addNotification, notifications } = useNotifications();

    return (
      <div>
        <div data-testid="notification-type">
          {notifications.length > 0 ? notifications[0].type : ''}
        </div>
        <button
          data-testid="add-notification"
          onClick={() =>
            addNotification({
              type,
              title: `Test ${type}`,
              message: `Test ${type} message`,
            })
          }
        >
          Add {type} Notification
        </button>
      </div>
    );
  };

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  test.each(['success', 'error', 'info', 'warning'] as const)('should handle %s notification type', (type) => {
    renderWithProviders(
      <NotificationProvider>
        <TypeTestComponent type={type} />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notification-type')).toHaveTextContent(type);
  });
});