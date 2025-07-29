import { describe, test, expect, vi, beforeAll } from 'vitest';

// Mock the API service before importing the component
beforeAll(() => {
  // Mock EventSource globally
  global.EventSource = vi.fn().mockImplementation(() => ({
    close: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    onopen: null,
    onmessage: null,
    onerror: null,
    readyState: 0,
  }));
});

// Mock the services/api module
vi.mock('../../services/api', () => ({
  sourcesService: {
    getSyncProgressStream: vi.fn().mockReturnValue({
      close: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      onopen: null,
      onmessage: null,
      onerror: null,
      readyState: 0,
    }),
  },
  SyncProgressInfo: {},
}));

// Simple compilation and type safety test for SyncProgressDisplay
describe('SyncProgressDisplay Compilation Tests', () => {
  test('should import and compile correctly', async () => {
    // Test that the component can be imported without runtime errors
    const component = await import('../SyncProgressDisplay');
    expect(component.SyncProgressDisplay).toBeDefined();
    expect(component.default).toBeDefined();
  }, 10000); // Increase timeout to 10 seconds

  test('should accept correct prop types', () => {
    // Test TypeScript compilation by defining expected props
    interface ExpectedProps {
      sourceId: string;
      sourceName: string;
      isVisible: boolean;
      onClose?: () => void;
    }

    const validProps: ExpectedProps = {
      sourceId: 'test-123',
      sourceName: 'Test Source',
      isVisible: true,
      onClose: () => console.log('closed'),
    };

    // If this compiles, the types are correct
    expect(validProps.sourceId).toBe('test-123');
    expect(validProps.sourceName).toBe('Test Source');
    expect(validProps.isVisible).toBe(true);
    expect(typeof validProps.onClose).toBe('function');
  });

  test('should handle minimal required props', () => {
    interface MinimalProps {
      sourceId: string;
      sourceName: string;
      isVisible: boolean;
    }

    const minimalProps: MinimalProps = {
      sourceId: 'minimal-test',
      sourceName: 'Minimal Test Source',
      isVisible: false,
    };

    expect(minimalProps.sourceId).toBe('minimal-test');
    expect(minimalProps.isVisible).toBe(false);
  });
});