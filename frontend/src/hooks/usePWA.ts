import { useState, useEffect } from 'react';

/**
 * Hook to detect if the app is running in PWA/standalone mode
 * @returns boolean indicating if running as installed PWA
 */
export const usePWA = (): boolean => {
  const [isPWA, setIsPWA] = useState(false);

  useEffect(() => {
    const checkPWAMode = () => {
      // Check if running in standalone mode (installed PWA)
      const isStandalone = window.matchMedia('(display-mode: standalone)').matches;
      // iOS Safari specific check
      const isIOSStandalone = (window.navigator as any).standalone === true;

      setIsPWA(isStandalone || isIOSStandalone);
    };

    checkPWAMode();

    // Listen for display mode changes
    const mediaQuery = window.matchMedia('(display-mode: standalone)');
    const handleChange = () => checkPWAMode();

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  return isPWA;
};
