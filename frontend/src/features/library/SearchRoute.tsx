import { Navigate, useLocation } from 'react-router-dom';
import { SearchPage } from './search/SearchPage';

/**
 * /search. The older `?query=` becomes `?q=` (every other parameter is kept); then the Search
 * page itself.
 */
export default function SearchRoute() {
  const { search, hash, state } = useLocation();
  const params = new URLSearchParams(search);
  if (!params.has('query')) return <SearchPage />;
  const q = params.get('q') ?? params.get('query');
  params.delete('query');
  params.delete('q');
  const next = new URLSearchParams();
  if (q) next.set('q', q);
  params.forEach((value, key) => next.append(key, value));
  const qs = next.toString();
  return <Navigate to={`/search${qs ? `?${qs}` : ''}${hash}`} replace state={state} />;
}
