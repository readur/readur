import { useEffect, useState } from 'react';
import { documentService, type Document, type OcrResponse } from '../../../services/api';

export type OcrLoadState = 'idle' | 'loading' | 'ready' | 'error';

export interface UseOcrTextResult {
  ocr: OcrResponse | null;
  state: OcrLoadState;
}

/**
 * Fetches the extracted text once the document has some. Re-fetches when the OCR status changes,
 * so text appears as soon as a running job finishes.
 */
export function useOcrText(document: Document | null): UseOcrTextResult {
  const [ocr, setOcr] = useState<OcrResponse | null>(null);
  const [state, setState] = useState<OcrLoadState>('idle');
  const id = document?.id;
  const hasText = Boolean(document?.has_ocr_text);
  const status = document?.ocr_status;

  useEffect(() => {
    if (!id || !hasText) {
      setOcr(null);
      setState('idle');
      return undefined;
    }
    let cancelled = false;
    setState((prev) => (prev === 'ready' ? prev : 'loading'));
    documentService
      .getOcrText(id)
      .then((res) => {
        if (cancelled) return;
        setOcr(res.data);
        setState('ready');
      })
      .catch(() => {
        if (!cancelled) setState('error');
      });
    return () => {
      cancelled = true;
    };
  }, [id, hasText, status]);

  return { ocr, state };
}
