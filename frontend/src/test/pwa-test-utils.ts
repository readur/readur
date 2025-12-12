import { vi } from 'vitest';

/**
 * Creates a matchMedia mock that can be configured for different query responses
 * @param standaloneMode - Whether to simulate PWA standalone mode
 * @returns Mock implementation of window.matchMedia
 */
export const createMatchMediaMock = (standaloneMode: boolean = false) => {
  return vi.fn().mockImplementation((query: string) => ({
    matches: query.includes('standalone') ? standaloneMode : false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(), // Deprecated but still supported
    removeListener: vi.fn(), // Deprecated but still supported
    dispatchEvent: vi.fn(),
  }));
};

/**
 * Sets up window.matchMedia to simulate PWA standalone mode
 * @param enabled - Whether PWA mode should be enabled (default: true)
 */
export const setupPWAMode = (enabled: boolean = true) => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createMatchMediaMock(enabled),
  });
};

/**
 * Sets up iOS-specific PWA detection via navigator.standalone
 * @param enabled - Whether iOS PWA mode should be enabled (default: true)
 */
export const setupIOSPWAMode = (enabled: boolean = true) => {
  Object.defineProperty(window.navigator, 'standalone', {
    writable: true,
    configurable: true,
    value: enabled,
  });
};

/**
 * Resets PWA-related window properties to their default state
 * Useful for cleanup between tests
 */
export const resetPWAMocks = () => {
  // Reset matchMedia to default non-PWA state
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createMatchMediaMock(false),
  });

  // Reset iOS standalone if it exists
  if ('standalone' in window.navigator) {
    Object.defineProperty(window.navigator, 'standalone', {
      writable: true,
      configurable: true,
      value: undefined,
    });
  }
};

/**
 * Creates a matchMedia mock that supports multiple query patterns
 * @param queries - Map of query patterns to their match states
 * @returns Mock implementation that responds to different queries
 *
 * @example
 * ```typescript
 * const mockFn = createResponsiveMatchMediaMock({
 *   'standalone': true,  // PWA mode
 *   'max-width: 900px': true,  // Mobile
 * });
 * ```
 */
export const createResponsiveMatchMediaMock = (
  queries: Record<string, boolean>
) => {
  return vi.fn().mockImplementation((query: string) => {
    // Check if any of the query patterns match the input query
    const matches = Object.entries(queries).some(([pattern, shouldMatch]) =>
      query.includes(pattern) ? shouldMatch : false
    );

    return {
      matches,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    };
  });
};
