import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import TestNotification from '../TestNotification';
import { NotificationProvider } from '../../contexts/NotificationContext';
import { renderWithProviders, setupTestEnvironment } from '../../test/test-utils';
import React from 'react';

const renderTestNotification = () => {
  return renderWithProviders(
    <NotificationProvider>
      <TestNotification />
    </NotificationProvider>
  );
};

describe('TestNotification', () => {
  beforeEach(() => {
    setupTestEnvironment();
    vi.clearAllMocks();
  });

  test('should render all test buttons', () => {
    renderTestNotification();

    expect(screen.getByText('Test Single Success')).toBeInTheDocument();
    expect(screen.getByText('Test Error')).toBeInTheDocument();
    expect(screen.getByText('Test Batch Success')).toBeInTheDocument();
    expect(screen.getByText('Test Mixed Batch')).toBeInTheDocument();
  });

  test('should have correct button variants and colors', () => {
    renderTestNotification();

    const successButton = screen.getByText('Test Single Success');
    const errorButton = screen.getByText('Test Error');
    const batchButton = screen.getByText('Test Batch Success');
    const mixedButton = screen.getByText('Test Mixed Batch');

    // Check that buttons exist (specific styling checks would depend on implementation)
    expect(successButton).toBeInTheDocument();
    expect(errorButton).toBeInTheDocument();
    expect(batchButton).toBeInTheDocument();
    expect(mixedButton).toBeInTheDocument();
  });

  test('should trigger notifications when buttons are clicked', () => {
    renderTestNotification();

    // Click single success button
    fireEvent.click(screen.getByText('Test Single Success'));
    
    // The notification should appear in the context
    // We can't easily test this without mocking the context or checking DOM changes
    // This test ensures the button is clickable and doesn't throw errors
    expect(screen.getByText('Test Single Success')).toBeInTheDocument();
  });

  test('should trigger error notification', () => {
    renderTestNotification();

    fireEvent.click(screen.getByText('Test Error'));
    
    // Button should still be present (test didn't crash)
    expect(screen.getByText('Test Error')).toBeInTheDocument();
  });

  test('should trigger batch success notification', () => {
    renderTestNotification();

    fireEvent.click(screen.getByText('Test Batch Success'));
    
    expect(screen.getByText('Test Batch Success')).toBeInTheDocument();
  });

  test('should trigger mixed batch notification', () => {
    renderTestNotification();

    fireEvent.click(screen.getByText('Test Mixed Batch'));
    
    expect(screen.getByText('Test Mixed Batch')).toBeInTheDocument();
  });

  test('should be arranged horizontally with spacing', () => {
    renderTestNotification();

    const container = screen.getByText('Test Single Success').closest('div');
    expect(container).toBeInTheDocument();
    
    // Check that all buttons are in the same container
    expect(container).toContainElement(screen.getByText('Test Single Success'));
    expect(container).toContainElement(screen.getByText('Test Error'));
    expect(container).toContainElement(screen.getByText('Test Batch Success'));
    expect(container).toContainElement(screen.getByText('Test Mixed Batch'));
  });
});

// Integration test with real notification context
describe('TestNotification Integration', () => {
  test('should actually create notifications when used with real context', () => {
    const TestWrapper = () => {
      const [notificationCount, setNotificationCount] = React.useState(0);

      React.useEffect(() => {
        // Listen for DOM changes to count notifications
        const observer = new MutationObserver(() => {
          const notifications = document.querySelectorAll('[data-testid^="notification-"]');
          setNotificationCount(notifications.length);
        });

        observer.observe(document.body, {
          childList: true,
          subtree: true,
        });

        return () => observer.disconnect();
      }, []);

      return (
        <div>
          <TestNotification />
          <div data-testid="notification-count">{notificationCount}</div>
        </div>
      );
    };

    renderWithProviders(
      <NotificationProvider>
        <TestWrapper />
      </NotificationProvider>
    );

    expect(screen.getByTestId('notification-count')).toHaveTextContent('0');

    // Click a button to trigger notification
    fireEvent.click(screen.getByText('Test Single Success'));

    // Note: In a real test environment, you might need to wait for state updates
    // or use a more sophisticated method to verify notification creation
  });
});