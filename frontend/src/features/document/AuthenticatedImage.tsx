import { useEffect, useRef, useState } from 'react';
import { Skeleton } from '../../ui';

export interface AuthenticatedImageProps {
  /** Fetches the image through the authenticated API client (`responseType: 'blob'`). */
  load: () => Promise<{ data: BlobPart }>;
  /** Identifies what `load` fetches (e.g. a document id); the image reloads when it changes. */
  resourceKey: string;
  alt: string;
  /** Shown when the image cannot be loaded. */
  unavailableText: string;
  className?: string;
  /** Class for the unavailable message. */
  messageClassName?: string;
}

type State = { status: 'loading' } | { status: 'failed' } | { status: 'ready'; url: string };

/**
 * An image served by an authenticated API endpoint. A plain `<img src="/api/…">` would not send
 * the bearer token, so the image is fetched as a blob and shown from an object URL.
 */
export function AuthenticatedImage({
  load,
  resourceKey,
  alt,
  unavailableText,
  className,
  messageClassName,
}: AuthenticatedImageProps) {
  const [state, setState] = useState<State>({ status: 'loading' });
  // `load` is usually an inline closure that changes on every render; keep the latest one
  // without making it an effect dependency.
  const loadRef = useRef(load);
  loadRef.current = load;

  useEffect(() => {
    let objectUrl: string | null = null;
    let cancelled = false;
    setState({ status: 'loading' });
    loadRef
      .current()
      .then((response) => {
        if (cancelled) return;
        const data = response.data;
        objectUrl = URL.createObjectURL(data instanceof Blob ? data : new Blob([data]));
        setState({ status: 'ready', url: objectUrl });
      })
      .catch(() => {
        if (!cancelled) setState({ status: 'failed' });
      });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [resourceKey]);

  if (state.status === 'failed') return <p className={messageClassName}>{unavailableText}</p>;
  if (state.status === 'loading') return <Skeleton height={200} label={alt} />;
  return <img className={className} src={state.url} alt={alt} onError={() => setState({ status: 'failed' })} />;
}

export default AuthenticatedImage;
