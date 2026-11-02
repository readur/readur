import { api, documentService } from './api'
import { isSafeInlineMime } from './contentSafety'

/**
 * Open a document in a new tab using the authenticated API client (a plain
 * link to /api/documents/... would not carry the bearer token). Types that a
 * browser could execute as active content are downloaded instead.
 */
export async function openDocumentInNewTab(id: string, filename?: string): Promise<void> {
  // Open the tab synchronously so popup blockers treat it as user-initiated.
  const newTab = window.open('', '_blank')
  if (newTab) newTab.opener = null
  try {
    const response = await api.get(`/documents/${id}/view`, { responseType: 'blob' })
    const mimeType = String(response.headers?.['content-type'] || '').split(';')[0].trim().toLowerCase()
    if (!newTab || !isSafeInlineMime(mimeType)) {
      newTab?.close()
      await documentService.downloadFile(id, filename)
      return
    }
    const url = window.URL.createObjectURL(new Blob([response.data], { type: mimeType }))
    newTab.location.href = url
    // Give the new tab time to load before releasing the blob.
    setTimeout(() => window.URL.revokeObjectURL(url), 60_000)
  } catch (error) {
    newTab?.close()
    throw error
  }
}

/** Fire-and-forget wrapper for click handlers. */
export const openDocumentInNewTabSafely = (id: string, filename?: string): void => {
  openDocumentInNewTab(id, filename).catch((err) => console.error('Failed to open document:', err))
}
