import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePWA } from '../usePWA';
import { setupPWAMode, setupIOSPWAMode, resetPWAMocks } from '../../test/pwa-test-utils';

describe('usePWA', () => {
  // Clean up after each test to prevent pollution
  afterEach(() => {
    resetPWAMocks();
  });

  describe('PWA Detection', () => {
    it('returns false when not in standalone mode', () => {
      // Setup: not in PWA mode
      setupPWAMode(false);

      const { result } = renderHook(() => usePWA());

      expect(result.current).toBe(false);
    });

    it('returns true when display-mode is standalone', () => {
      // Setup: PWA mode via display-mode
      setupPWAMode(true);

      const { result } = renderHook(() => usePWA());

      expect(result.current).toBe(true);
    });

    it('returns true when navigator.standalone is true (iOS)', () => {
      // Setup: iOS PWA mode (not using matchMedia)
      setupPWAMode(false); // matchMedia returns false
      setupIOSPWAMode(true); // But iOS standalone is true

      const { result } = renderHook(() => usePWA());

      expect(result.current).toBe(true);
    });

    it('returns true when both display-mode and iOS standalone are true', () => {
      // Setup: Both detection methods return true
      setupPWAMode(true);
      setupIOSPWAMode(true);

      const { result } = renderHook(() => usePWA());

      expect(result.current).toBe(true);
    });
  });

  describe('Event Listener Management', () => {
    it('registers event listener on mount', () => {
      const addEventListener = vi.fn();
      const removeEventListener = vi.fn();

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn().mockImplementation(() => ({
          matches: false,
          media: '(display-mode: standalone)',
          addEventListener,
          removeEventListener,
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      renderHook(() => usePWA());

      expect(addEventListener).toHaveBeenCalledWith('change', expect.any(Function));
    });

    it('removes event listener on unmount', () => {
      const addEventListener = vi.fn();
      const removeEventListener = vi.fn();

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn().mockImplementation(() => ({
          matches: false,
          media: '(display-mode: standalone)',
          addEventListener,
          removeEventListener,
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      const { unmount } = renderHook(() => usePWA());

      // Capture the registered handler
      const registeredHandler = addEventListener.mock.calls[0][1];

      unmount();

      expect(removeEventListener).toHaveBeenCalledWith('change', registeredHandler);
    });

    it('handles multiple mount/unmount cycles correctly', () => {
      setupPWAMode(false);

      // First mount
      const { unmount: unmount1 } = renderHook(() => usePWA());
      unmount1();

      // Second mount (should not cause errors)
      const { result: result2, unmount: unmount2 } = renderHook(() => usePWA());
      expect(result2.current).toBe(false);
      unmount2();

      // Third mount with PWA enabled
      setupPWAMode(true);
      const { result: result3 } = renderHook(() => usePWA());
      expect(result3.current).toBe(true);
    });
  });

  describe('Display Mode Changes', () => {
    it('updates state when display-mode changes', () => {
      let matchesValue = false;
      const listeners: Array<() => void> = [];

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn().mockImplementation(() => ({
          get matches() {
            return matchesValue;
          },
          media: '(display-mode: standalone)',
          addEventListener: vi.fn((event: string, handler: () => void) => {
            listeners.push(handler);
          }),
          removeEventListener: vi.fn(),
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      const { result, rerender } = renderHook(() => usePWA());

      // Initially not in PWA mode
      expect(result.current).toBe(false);

      // Simulate entering PWA mode
      act(() => {
        matchesValue = true;
        // Trigger the change event
        listeners.forEach(handler => handler());
      });
      rerender();

      // Should now detect PWA mode
      expect(result.current).toBe(true);
    });

    it('updates state when exiting PWA mode', () => {
      let matchesValue = true;
      const listeners: Array<() => void> = [];

      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: vi.fn().mockImplementation(() => ({
          get matches() {
            return matchesValue;
          },
          media: '(display-mode: standalone)',
          addEventListener: vi.fn((event: string, handler: () => void) => {
            listeners.push(handler);
          }),
          removeEventListener: vi.fn(),
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      const { result, rerender } = renderHook(() => usePWA());

      // Initially in PWA mode
      expect(result.current).toBe(true);

      // Simulate exiting PWA mode
      act(() => {
        matchesValue = false;
        // Trigger the change event
        listeners.forEach(handler => handler());
      });
      rerender();

      // Should now detect non-PWA mode
      expect(result.current).toBe(false);
    });
  });

  describe('Edge Cases', () => {
    it('handles missing navigator.standalone gracefully', () => {
      // Setup matchMedia to return false
      setupPWAMode(false);

      // Ensure navigator.standalone is undefined
      const originalStandalone = (window.navigator as any).standalone;
      delete (window.navigator as any).standalone;

      const { result } = renderHook(() => usePWA());

      expect(result.current).toBe(false);

      // Restore original value if it existed
      if (originalStandalone !== undefined) {
        (window.navigator as any).standalone = originalStandalone;
      }
    });
  });

  describe('Consistency', () => {
    it('returns the same value on re-renders if conditions unchanged', () => {
      setupPWAMode(true);

      const { result, rerender } = renderHook(() => usePWA());

      expect(result.current).toBe(true);

      // Re-render multiple times
      rerender();
      expect(result.current).toBe(true);

      rerender();
      expect(result.current).toBe(true);
    });

    it('maintains state across re-renders', () => {
      setupPWAMode(false);

      const { result, rerender } = renderHook(() => usePWA());

      expect(result.current).toBe(false);

      rerender();
      expect(result.current).toBe(false);
    });
  });
});
