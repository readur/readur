import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import { NotificationProvider, useNotifications } from '../NotificationContext';
import { NotificationType } from '../../types/notification';
import React from 'react';

// Mock component to test the context
const TestComponent: React.FC = () => {
  const {
    notifications,
    unreadCount,
    addNotification,
    markAsRead,
    markAllAsRead,
    clearNotification,
    clearAll,
    addBatchNotification,
  } = useNotifications();

  return (
    <div>
      <div data-testid="unread-count">{unreadCount}</div>
      <div data-testid="notifications-count">{notifications.length}</div>
      <div data-testid="notifications">
        {notifications.map((notification) => (
          <div key={notification.id} data-testid={`notification-${notification.id}`}>
            <span data-testid={`title-${notification.id}`}>{notification.title}</span>
            <span data-testid={`message-${notification.id}`}>{notification.message}</span>
            <span data-testid={`type-${notification.id}`}>{notification.type}</span>
            <span data-testid={`read-${notification.id}`}>{notification.read.toString()}</span>
          </div>
        ))}
      </div>
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
      <button
        data-testid="add-batch"
        onClick={() =>
          addBatchNotification('success', 'upload', [
            { name: 'file1.pdf', success: true },
            { name: 'file2.pdf', success: true },
          ])
        }
      >
        Add Batch
      </button>
      <button
        data-testid="mark-first-read"
        onClick={() => {
          if (notifications.length > 0) {
            markAsRead(notifications[0].id);
          }
        }}
      >
        Mark First Read
      </button>
      <button data-testid="mark-all-read" onClick={markAllAsRead}>
        Mark All Read
      </button>
      <button
        data-testid="clear-first"
        onClick={() => {
          if (notifications.length > 0) {
            clearNotification(notifications[0].id);
          }
        }}
      >
        Clear First
      </button>
      <button data-testid="clear-all" onClick={clearAll}>
        Clear All
      </button>
    </div>
  );
};

const renderWithProvider = () => {
  return render(
    <NotificationProvider>
      <TestComponent />
    </NotificationProvider>
  );
};

describe('NotificationContext', () => {
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

  test('should add a single notification', async () => {
    renderWithProvider();
    
    const addButton = screen.getByTestId('add-notification');
    
    act(() => {
      addButton.click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');
    
    // Find the first notification element
    const notificationsContainer = screen.getByTestId('notifications');
    const firstNotification = notificationsContainer.firstElementChild as HTMLElement;
    expect(firstNotification).toBeTruthy();
    
    const notificationId = firstNotification.getAttribute('data-testid')?.replace('notification-', '');
    expect(notificationId).toBeTruthy();
    
    expect(screen.getByTestId(`title-${notificationId}`)).toHaveTextContent('Test');
    expect(screen.getByTestId(`message-${notificationId}`)).toHaveTextContent('Test message');
    expect(screen.getByTestId(`type-${notificationId}`)).toHaveTextContent('success');
    expect(screen.getByTestId(`read-${notificationId}`)).toHaveTextContent('false');
  });

  test('should mark notification as read', async () => {
    renderWithProvider();
    
    // Add a notification first
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');

    // Mark as read
    act(() => {
      screen.getByTestId('mark-first-read').click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
  });

  test('should mark all notifications as read', async () => {
    renderWithProvider();
    
    // Add multiple notifications
    act(() => {
      screen.getByTestId('add-notification').click();
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('2');

    // Mark all as read
    act(() => {
      screen.getByTestId('mark-all-read').click();
    });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('2');
  });

  test('should clear a single notification', async () => {
    renderWithProvider();
    
    // Add a notification
    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');

    // Clear the notification
    act(() => {
      screen.getByTestId('clear-first').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
  });

  test('should clear all notifications', async () => {
    renderWithProvider();
    
    // Add multiple notifications
    act(() => {
      screen.getByTestId('add-notification').click();
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('2');

    // Clear all
    act(() => {
      screen.getByTestId('clear-all').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('0');
  });

  test('should handle batch notifications with batching window', async () => {
    renderWithProvider();
    
    // Add batch notification
    act(() => {
      screen.getByTestId('add-batch').click();
    });

    // No notification should appear immediately (batching window)
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');

    // Fast forward time to trigger batch completion
    act(() => {
      vi.advanceTimersByTime(2100); // Slightly more than BATCH_WINDOW_MS (2000ms)
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');
  }, 15000);

  test('should batch multiple operations of same type', async () => {
    renderWithProvider();
    
    // Add multiple batch notifications quickly
    act(() => {
      screen.getByTestId('add-batch').click();
      screen.getByTestId('add-batch').click();
    });

    // No notifications should appear immediately
    expect(screen.getByTestId('notifications-count')).toHaveTextContent('0');

    // Fast forward time
    act(() => {
      vi.advanceTimersByTime(2100);
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    // Should only have one batched notification
    expect(screen.getByTestId('unread-count')).toHaveTextContent('1');
  }, 15000);

  test('should limit notifications to MAX_NOTIFICATIONS', async () => {
    renderWithProvider();
    
    // Add 52 notifications (more than MAX_NOTIFICATIONS = 50)
    act(() => {
      for (let i = 0; i < 52; i++) {
        screen.getByTestId('add-notification').click();
      }
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('50');
    expect(screen.getByTestId('unread-count')).toHaveTextContent('50');
  });

  test('should throw error when used outside provider', () => {
    // Mock console.error to avoid noisy test output
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    expect(() => {
      render(<TestComponent />);
    }).toThrow('useNotifications must be used within NotificationProvider');
    
    consoleSpy.mockRestore();
  });
});

// Test different notification types
describe('NotificationContext - Notification Types', () => {
  const notificationTypes: NotificationType[] = ['success', 'error', 'info', 'warning'];

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  test.each(notificationTypes)('should handle %s notification type', (type) => {
    const TestTypeComponent: React.FC = () => {
      const { addNotification, notifications } = useNotifications();

      return (
        <div>
          <div data-testid="notifications-count">{notifications.length}</div>
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

    render(
      <NotificationProvider>
        <TestTypeComponent />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('add-notification').click();
    });

    expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    expect(screen.getByTestId('notification-type')).toHaveTextContent(type);
  });
});

// Test batch notification scenarios
describe('NotificationContext - Batch Notifications', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const BatchTestComponent: React.FC = () => {
    const { addBatchNotification, notifications } = useNotifications();

    return (
      <div>
        <div data-testid="notifications-count">{notifications.length}</div>
        <div data-testid="notification-title">
          {notifications.length > 0 ? notifications[0].title : ''}
        </div>
        <div data-testid="notification-message">
          {notifications.length > 0 ? notifications[0].message : ''}
        </div>
        <button
          data-testid="single-success"
          onClick={() =>
            addBatchNotification('success', 'upload', [
              { name: 'document.pdf', success: true },
            ])
          }
        >
          Single Success
        </button>
        <button
          data-testid="batch-success"
          onClick={() =>
            addBatchNotification('success', 'upload', [
              { name: 'doc1.pdf', success: true },
              { name: 'doc2.pdf', success: true },
              { name: 'doc3.pdf', success: true },
            ])
          }
        >
          Batch Success
        </button>
        <button
          data-testid="mixed-batch"
          onClick={() =>
            addBatchNotification('warning', 'upload', [
              { name: 'doc1.pdf', success: true },
              { name: 'doc2.pdf', success: false },
              { name: 'doc3.pdf', success: true },
            ])
          }
        >
          Mixed Batch
        </button>
        <button
          data-testid="all-failed"
          onClick={() =>
            addBatchNotification('error', 'upload', [
              { name: 'doc1.pdf', success: false },
              { name: 'doc2.pdf', success: false },
            ])
          }
        >
          All Failed
        </button>
      </div>
    );
  };

  test('should create single file notification for one file', async () => {
    render(
      <NotificationProvider>
        <BatchTestComponent />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('single-success').click();
    });

    act(() => {
      vi.advanceTimersByTime(2100);
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    expect(screen.getByTestId('notification-title')).toHaveTextContent('File Uploaded');
    expect(screen.getByTestId('notification-message')).toHaveTextContent('document.pdf uploaded successfully');
  }, 15000);

  test('should create batch notification for multiple files', async () => {
    render(
      <NotificationProvider>
        <BatchTestComponent />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('batch-success').click();
    });

    act(() => {
      vi.advanceTimersByTime(2100);
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    expect(screen.getByTestId('notification-title')).toHaveTextContent('Batch Upload Complete');
    expect(screen.getByTestId('notification-message')).toHaveTextContent('3 files uploaded successfully');
  }, 15000);

  test('should handle mixed success/failure batch', async () => {
    render(
      <NotificationProvider>
        <BatchTestComponent />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('mixed-batch').click();
    });

    act(() => {
      vi.advanceTimersByTime(2100);
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    expect(screen.getByTestId('notification-title')).toHaveTextContent('Batch Upload Complete');
    expect(screen.getByTestId('notification-message')).toHaveTextContent('2 files uploaded, 1 failed');
  }, 15000);

  test('should handle all failed batch', async () => {
    render(
      <NotificationProvider>
        <BatchTestComponent />
      </NotificationProvider>
    );

    act(() => {
      screen.getByTestId('all-failed').click();
    });

    act(() => {
      vi.advanceTimersByTime(2100);
    });

    await waitFor(() => {
      expect(screen.getByTestId('notifications-count')).toHaveTextContent('1');
    }, { timeout: 10000 });

    expect(screen.getByTestId('notification-title')).toHaveTextContent('Batch Upload Complete');
    expect(screen.getByTestId('notification-message')).toHaveTextContent('Failed to upload 2 files');
  }, 15000);
});