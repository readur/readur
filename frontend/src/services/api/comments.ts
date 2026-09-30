import api from './client'

// ─── Comments Service ────────────────────────────────────────────────────

export interface CommentWithAuthor {
  id: string
  document_id: string
  user_id: string
  parent_id: string | null
  content: string
  is_edited: boolean
  created_at: string
  updated_at: string
  username: string
  user_role: string
}

export interface CommentThread {
  id: string
  document_id: string
  user_id: string
  parent_id: string | null
  content: string
  is_edited: boolean
  created_at: string
  updated_at: string
  username: string
  user_role: string
  reply_count: number
  replies: CommentWithAuthor[]
}

export interface CreateCommentRequest {
  content: string
  parent_id?: string
}

export interface UpdateCommentRequest {
  content: string
}

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
