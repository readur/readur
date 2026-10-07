import { Navigate, useLocation, useParams } from 'react-router-dom';

/**
 * /documents/:id from before the document drawer: the Library (or, with a search, the Search
 * page) with the drawer open on that document. Other params come along.
 */
export function DocumentRedirect() {
  const { id = '' } = useParams<{ id: string }>();
  const { search, hash } = useLocation();
  const params = new URLSearchParams(search);
  params.delete('document');
  params.append('document', id);
  const pathname = params.get('q')?.trim() ? '/search' : '/documents';
  return <Navigate replace to={{ pathname, search: `?${params.toString()}`, hash }} />;
}
