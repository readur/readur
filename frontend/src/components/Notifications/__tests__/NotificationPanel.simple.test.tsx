import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';
import NotificationPanel from '../NotificationPanel';
import { NotificationProvider } from '../../../contexts/NotificationContext';
import { renderWithProviders, setupTestEnvironment } from '../../../test/test-utils';
import React from 'react';

// Mock date-fns
vi.mock('date-fns', () => ({
  formatDistanceToNow: vi.fn(() => '2 minutes ago'),
}));


const createMockAnchorEl = () => {
  const mockEl = document.createElement('div');
  Object.defineProperty(mockEl, 'getBoundingClientRect', {
    value: () => ({
      bottom: 100,
      top: 50,
      left: 200,
      right: 250,
      width: 50,
      height: 50,
    }),
  });
  return mockEl;
};

describe('NotificationPanel - Simple Tests', () => {
  beforeEach(() => {
    setupTestEnvironment();
  });
  test('should not render when anchorEl is null', () => {
    const { container } = renderWithProviders(
      <NotificationProvider>
        <NotificationPanel anchorEl={null} onClose={vi.fn()} />
      </NotificationProvider>
    );

    expect(container.firstChild).toBeNull();
  });

  test('should render notification panel with header when anchorEl is provided', () => {
    const mockAnchorEl = createMockAnchorEl();

    renderWithProviders(
      <NotificationProvider>
        <NotificationPanel anchorEl={mockAnchorEl} onClose={vi.fn()} />
      </NotificationProvider>
    );

    expect(screen.getByText('Notifications')).toBeInTheDocument();
  });

  test('should show empty state when no notifications', () => {
    const mockAnchorEl = createMockAnchorEl();

    renderWithProviders(
      <NotificationProvider>
        <NotificationPanel anchorEl={mockAnchorEl} onClose={vi.fn()} />
      </NotificationProvider>
    );

    expect(screen.getByText('No notifications')).toBeInTheDocument();
  });

  test('should render with theme provider correctly', () => {
    const mockAnchorEl = createMockAnchorEl();

    const { container } = renderWithProviders(
      <NotificationProvider>
        <NotificationPanel anchorEl={mockAnchorEl} onClose={vi.fn()} />
      </NotificationProvider>
    );

    // Should render without crashing
    expect(container.firstChild).toBeTruthy();
  });
});