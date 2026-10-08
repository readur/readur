import api from './client'
import type { CreateLabel, Label, UpdateLabel } from '../../types/generated'

export type { CreateLabel, Label as LabelResponse, UpdateLabel } from '../../types/generated'

/** How `labelService.bulkAssign` combines the given labels with each document's current ones. */
export type BulkLabelMode = 'add' | 'remove' | 'replace'

export const labelService = {
  /** GET /labels. Counts cost an extra join, so they are off unless asked for. */
  list: (includeCounts = false) => {
    return api.get<Label[]>('/labels', { params: { include_counts: includeCounts } })
  },

  create: (data: CreateLabel) => {
    return api.post<Label>('/labels', data)
  },

  update: (id: string, data: UpdateLabel) => {
    return api.put<Label>(`/labels/${id}`, data)
  },

  /** GET /labels/documents/:id — the labels on one document. */
  getDocumentLabels: (documentId: string) => {
    return api.get<Label[]>(`/labels/documents/${documentId}`)
  },

  /** Replace a document's labels with exactly `labelIds`. */
  setDocumentLabels: (documentId: string, labelIds: string[]) => {
    return api.put(`/labels/documents/${documentId}`, { label_ids: labelIds })
  },

  /** Apply labels to many documents in one call (POST /labels/bulk/documents). */
  bulkAssign: (documentIds: string[], labelIds: string[], mode: BulkLabelMode = 'add') => {
    return api.post('/labels/bulk/documents', {
      document_ids: documentIds,
      label_ids: labelIds,
      mode,
    })
  },
}
