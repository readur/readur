import { ocrService, type FailedDocumentRow } from '../../../services/api';
import { FAILURE_STAGES } from './failureLabels';

/** OCR failures are listed with their documents; this list covers every other stage. */
export const NON_OCR_STAGES = FAILURE_STAGES.filter((s) => s !== 'ocr');

export interface ImportFailuresPage {
  rows: FailedDocumentRow[];
  total: number;
}

export interface ImportFailuresQuery {
  /** One non-OCR stage, or undefined for all of them. */
  stage?: string;
  reason?: string;
  limit: number;
  offset: number;
}

/**
 * Failed import records outside OCR, paged consistently. GET /documents/failed filters by one
 * exact stage only, so "all stages" is served stage by stage: the totals of each stage are read
 * first, then only the stages that overlap the requested window are fetched, in stage order.
 * Nothing is dropped on the client, so page sizes and totals always agree with the server.
 */
export async function fetchImportFailures({ stage, reason, limit, offset }: ImportFailuresQuery): Promise<ImportFailuresPage> {
  if (stage) {
    const res = await ocrService.listFailedDocuments({ stage, reason, limit, offset });
    return { rows: res.data?.documents ?? [], total: res.data?.pagination?.total ?? 0 };
  }

  const totals = await Promise.all(
    NON_OCR_STAGES.map(async (s) => {
      const res = await ocrService.listFailedDocuments({ stage: s, reason, limit: 1, offset: 0 });
      return res.data?.pagination?.total ?? 0;
    }),
  );
  const total = totals.reduce((a, b) => a + b, 0);

  const slices: Array<Promise<FailedDocumentRow[]>> = [];
  let start = 0;
  NON_OCR_STAGES.forEach((s, i) => {
    const end = start + totals[i];
    const from = Math.max(offset, start);
    const to = Math.min(offset + limit, end);
    if (to > from) {
      slices.push(
        ocrService
          .listFailedDocuments({ stage: s, reason, limit: to - from, offset: from - start })
          .then((res) => res.data?.documents ?? []),
      );
    }
    start = end;
  });
  const rows = (await Promise.all(slices)).flat();
  return { rows, total };
}
