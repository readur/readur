import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import SyncProgressDisplay from '../SyncProgressDisplay';
import { renderWithProviders } from '../../test/test-utils';

// Simple mock EventSource that focuses on essential functionality
const createMockEventSource = () => ({
  onopen: null as ((event: Event) => void) | null,
  onmessage: null as ((event: MessageEvent) => void) | null,
  onerror: null as ((event: Event) => void) | null,
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  close: vi.fn(),
  readyState: 0, // CONNECTING
  url: '',
  withCredentials: false,
  dispatchEvent: vi.fn(),
});

// Mock sourcesService 
const mockSourcesService = {
  getSyncProgressStream: vi.fn(),
  getSyncStatus: vi.fn(),
  triggerSync: vi.fn(),
  stopSync: vi.fn(),
  triggerDeepScan: vi.fn(),
};

// Mock the API - ensure EventSource is mocked first
global.EventSource = vi.fn(() => createMockEventSource()) as any;
(global.EventSource as any).CONNECTING = 0;
(global.EventSource as any).OPEN = 1;
(global.EventSource as any).CLOSED = 2;

vi.mock('../../services/api', () => ({
  sourcesService: mockSourcesService,
}));

const renderComponent = (props = {}) => {
  const defaultProps = {
    sourceId: 'test-source-123',
    sourceName: 'Test WebDAV Source',
    isVisible: true,
    ...props,
  };

  return renderWithProviders(<SyncProgressDisplay {...defaultProps} />);
};

describe('SyncProgressDisplay Simple Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSourcesService.getSyncProgressStream.mockReturnValue(createMockEventSource());
  });

  describe('Basic Rendering', () => {
    test('should not render when isVisible is false', () => {
      renderComponent({ isVisible: false });
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });

    test('should render title when isVisible is true', () => {
      renderComponent({ isVisible: true });
      expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
    });

    test('should render with custom source name', () => {
      renderComponent({ sourceName: 'My Custom Source' });
      expect(screen.getByText('My Custom Source - Sync Progress')).toBeInTheDocument();
    });

    test('should show initial waiting message', () => {
      renderComponent();
      expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
    });
  });

  describe('EventSource Connection', () => {
    test('should create EventSource with correct source ID', () => {
      renderComponent({ sourceId: 'test-123' });
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('test-123');
    });

    test('should create EventSource only when visible', () => {
      renderComponent({ isVisible: false });
      expect(mockSourcesService.getSyncProgressStream).not.toHaveBeenCalled();
    });

    test('should show connecting status initially', () => {
      renderComponent();
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });
  });

  describe('Expand/Collapse', () => {
    test('should have expand/collapse button', () => {
      renderComponent();
      // Look for the expand/collapse button by its tooltip or aria-label
      const buttons = screen.getAllByRole('button');
      const expandCollapseButton = buttons.find(button => 
        button.getAttribute('aria-label')?.includes('Collapse') ||
        button.getAttribute('aria-label')?.includes('Expand')
      );
      expect(expandCollapseButton).toBeInTheDocument();
    });

    test('should toggle expansion when button is clicked', async () => {
      renderComponent();
      
      // Find the expand/collapse button
      const buttons = screen.getAllByRole('button');
      const expandCollapseButton = buttons.find(button => 
        button.getAttribute('aria-label')?.includes('Collapse')
      );
      
      if (expandCollapseButton) {
        // Should be expanded initially
        expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
        
        // Click to collapse
        fireEvent.click(expandCollapseButton);
        
        await waitFor(() => {
          expect(screen.queryByText('Waiting for sync progress information...')).not.toBeInTheDocument();
        });
      }
    });
  });

  describe('Component Cleanup', () => {
    test('should close EventSource on unmount', () => {
      const mockEventSource = createMockEventSource();
      mockSourcesService.getSyncProgressStream.mockReturnValue(mockEventSource);
      
      const { unmount } = renderComponent();
      unmount();
      
      expect(mockEventSource.close).toHaveBeenCalled();
    });

    test('should close EventSource when visibility changes to false', () => {
      const mockEventSource = createMockEventSource();
      mockSourcesService.getSyncProgressStream.mockReturnValue(mockEventSource);
      
      const { rerender } = renderComponent({ isVisible: true });
      
      rerender(
        <SyncProgressDisplay
          sourceId="test-source-123"
          sourceName="Test WebDAV Source"
          isVisible={false}
        />
      );
      
      expect(mockEventSource.close).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    test('should handle EventSource creation errors gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockSourcesService.getSyncProgressStream.mockImplementation(() => {
        throw new Error('Failed to create EventSource');
      });

      // Should not crash when EventSource creation fails
      expect(() => renderComponent()).not.toThrow();
      
      consoleErrorSpy.mockRestore();
    });
  });

  describe('Props Validation', () => {
    test('should handle different source IDs', () => {
      renderComponent({ sourceId: 'different-id' });
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('different-id');
    });

    test('should handle empty source name', () => {
      renderComponent({ sourceName: '' });
      expect(screen.getByText(' - Sync Progress')).toBeInTheDocument();
    });

    test('should handle very long source names', () => {
      const longName = 'A'.repeat(100);
      renderComponent({ sourceName: longName });
      expect(screen.getByText(`${longName} - Sync Progress`)).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    test('should have proper heading structure', () => {
      renderComponent();
      const heading = screen.getByText('Test WebDAV Source - Sync Progress');
      expect(heading.tagName).toBe('H6'); // Material-UI Typography variant="h6"
    });

    test('should have clickable buttons with proper attributes', () => {
      renderComponent();
      const buttons = screen.getAllByRole('button');
      
      buttons.forEach(button => {
        expect(button).toHaveAttribute('type', 'button');
      });
    });
  });
});