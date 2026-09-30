import { Navigate, useLocation } from 'react-router-dom';

/**
 * /search now lives in the Library. `?q=` or the older `?query=` becomes `q`; every other
 * parameter is kept.
 */
export default function SearchRoute() {
  const { search, hash } = useLocation();
  const params = new URLSearchParams(search);
  const q = params.get('q') ?? params.get('query');
  params.delete('query');
  params.delete('q');
  const next = new URLSearchParams();
  if (q) next.set('q', q);
  params.forEach((value, key) => next.append(key, value));
  const qs = next.toString();
  return <Navigate to={`/documents${qs ? `?${qs}` : ''}${hash}`} replace />;
}
