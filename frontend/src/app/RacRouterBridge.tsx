import type { ReactNode } from 'react';
import { RouterProvider } from 'react-aria-components';
import { useHref, useNavigate, type NavigateOptions } from 'react-router-dom';

declare module 'react-aria-components' {
  interface RouterConfig {
    routerOptions: NavigateOptions;
  }
}

/**
 * Lets React Aria links (ButtonLink, DocumentCard's `href`, menu items with `href`) navigate
 * through React Router instead of reloading the page. Sits inside the router. A link's
 * `routerOptions` (history state, replace) go along with it.
 */
export function RacRouterBridge({ children }: { children: ReactNode }) {
  const navigate = useNavigate();
  return (
    <RouterProvider navigate={(to, options) => navigate(to, options)} useHref={(to) => useHref(to)}>
      {children}
    </RouterProvider>
  );
}
