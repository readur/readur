import { useEffect, useState } from 'react';
import { documentService, type OcrResponse } from '../../services/api';

export const EXCERPT_LENGTH = 600;

export interface OcrExcerpt {
  text: string | null;
  truncated: boolean;
  language: string | null;
  pages: number | null;
  confidence: number | null;
}

type State = { status: 'loading' } | { status: 'ready'; excerpt: OcrExcerpt } | { status: 'error' };

function toExcerpt(ocr: OcrResponse): OcrExcerpt {
  const full = (ocr.ocr_text ?? '').trim();
  return {
    text: full ? full.slice(0, EXCERPT_LENGTH) : null,
    truncated: full.length > EXCERPT_LENGTH,
    language: ocr.detected_language ?? null,
    pages: ocr.pages_processed ?? null,
    confidence: ocr.ocr_confidence ?? null,
  };
}

/**
 * The start of a document's OCR text plus OCR facts. `cache` belongs to the caller (the panel)
 * so stepping back and forth with the arrow keys does not refetch; after dropping an entry,
 * bump `epoch` to load it again.
 */
export function useOcrExcerpt(documentId: string | null, cache: Map<string, OcrExcerpt>, epoch = 0): State {
  const [state, setState] = useState<State>({ status: 'loading' });

  useEffect(() => {
    if (!documentId) return undefined;
    const cached = cache.get(documentId);
    if (cached) {
      setState({ status: 'ready', excerpt: cached });
      return undefined;
    }
    let live = true;
    setState({ status: 'loading' });
    documentService
      .getOcrText(documentId)
      .then((res) => {
        const excerpt = toExcerpt(res.data);
        cache.set(documentId, excerpt);
        if (live) setState({ status: 'ready', excerpt });
      })
      .catch(() => {
        if (live) setState({ status: 'error' });
      });
    return () => {
      live = false;
    };
  }, [documentId, cache, epoch]);

  return state;
}
