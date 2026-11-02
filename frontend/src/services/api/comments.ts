import api from './client'
import type {
  CommentThread,
  CommentWithAuthor,
  CreateCommentRequest,
  UpdateCommentRequest,
} from '../../types/generated'

export type {
  CommentThread,
  CommentWithAuthor,
  CreateCommentRequest,
  UpdateCommentRequest,
} from '../../types/generated'

// ─── Comments Service ────────────────────────────────────────────────────

export const commentsService = {
  list: (documentId: string, limit = 50, offset = 0) => {
    return api.get<CommentThread[]>(`/comments/documents/${documentId}/comments`, {
      params: { limit, offset },
    })
  },

  getReplies: (documentId: string, commentId: string, limit = 50, offset = 0) => {
    return api.get<CommentWithAuthor[]>(
      `/comments/documents/${documentId}/comments/${commentId}/replies`,
      { params: { limit, offset } }
    )
  },

  create: (documentId: string, request: CreateCommentRequest) => {
    return api.post<CommentWithAuthor>(`/comments/documents/${documentId}/comments`, request)
  },

  update: (documentId: string, commentId: string, request: UpdateCommentRequest) => {
    return api.put<CommentWithAuthor>(
      `/comments/documents/${documentId}/comments/${commentId}`,
      request
    )
  },

  delete: (documentId: string, commentId: string) => {
    return api.delete(`/comments/documents/${documentId}/comments/${commentId}`)
  },

  getCount: (documentId: string) => {
    return api.get<{ count: number }>(`/comments/documents/${documentId}/comments/count`)
  },
}
