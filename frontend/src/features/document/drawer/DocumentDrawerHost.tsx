import { lazy, Suspense, useEffect } from 'react';
import { useLastDefined } from '../../../lib/useLastDefined';
import { useDocumentDrawer, useDocumentList } from './DocumentDrawerContext';

const loadDrawer = () => import('./DocumentDrawer');
const DocumentDrawer = lazy(loadDrawer);

/**
 * Renders the document drawer for `?document=` on whatever page is showing. Its code loads on
 * first use, or ahead of time once a page with a document list is on screen.
 */
export function DocumentDrawerHost() {
  const drawer = useDocumentDrawer();
  const list = useDocumentList();
  const shown = useLastDefined(drawer.id);

  useEffect(() => {
    if (list) void loadDrawer();
  }, [list]);

  if (!shown) return null;
  return (
    <Suspense fallback={null}>
      <DocumentDrawer id={shown} isOpen={drawer.id !== null} onClose={drawer.close} onStep={drawer.step} />
    </Suspense>
  );
}
