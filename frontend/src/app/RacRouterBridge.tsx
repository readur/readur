import type { ReactNode } from 'react';
import { RouterProvider } from 'react-aria-components';
import { useHref, useNavigate } from 'react-router-dom';

/**
 * Lets React Aria links (ButtonLink, DocumentCard's `href`, menu items with `href`) navigate
 * through React Router instead of reloading the page. Sits inside the router.
 */
export function RacRouterBridge({ children }: { children: ReactNode }) {
  const navigate = useNavigate();
  return (
    <RouterProvider navigate={(to) => navigate(to)} useHref={(to) => useHref(to)}>
      {children}
    </RouterProvider>
  );
}
