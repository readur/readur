import { Navigate, useLocation } from 'react-router-dom';
import { mergeSearch } from './legacyRoutes';

/** Replaces the current (old) URL with `to`, keeping the old query string and hash. */
export function LegacyRedirect({ to }: { to: string }) {
  const { search, hash } = useLocation();
  return <Navigate replace to={{ ...mergeSearch(to, search), hash }} />;
}

export default LegacyRedirect;
