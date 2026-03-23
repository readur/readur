import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Box,
  Typography,
  CircularProgress,
  Alert,
  Divider,
} from '@mui/material';
import { ChatBubbleOutline as CommentIcon } from '@mui/icons-material';
import {
  commentsService,
  type CommentThread as CommentThreadType,
  type CommentWithAuthor,
} from '../../services/api';
import { useAuth } from '../../contexts/AuthContext';
import CommentForm from '../Comments/CommentForm';
import CommentThread from '../Comments/CommentThread';

interface CommentsTabProps {
  documentId: string;
}

const POLL_INTERVAL = 30_000;

const CommentsTab: React.FC<CommentsTabProps> = ({ documentId }) => {
  const { user } = useAuth();
  const [threads, setThreads] = useState<CommentThreadType[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const pollTimer = useRef<ReturnType<typeof setInterval>>();

  const fetchComments = useCallback(async () => {
    try {
      const response = await commentsService.list(documentId);
      setThreads(response.data);
      setError(null);
    } catch {
      setError('Failed to load comments.');
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    fetchComments();
    pollTimer.current = setInterval(fetchComments, POLL_INTERVAL);
    return () => {
      if (pollTimer.current) clearInterval(pollTimer.current);
    };
  }, [fetchComments]);

  const handleCreateComment = async (content: string) => {
    if (!user) return;

    // Optimistic: build a temporary thread from the new comment
    const optimistic: CommentThreadType = {
      id: `temp-${Date.now()}`,
      document_id: documentId,
      user_id: user.id,
      parent_id: null,
      content,
      is_edited: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      username: user.username,
      user_role: user.role,
      reply_count: 0,
      replies: [],
    };

    setThreads((prev) => [optimistic, ...prev]);

    try {
      await commentsService.create(documentId, { content });
      // Re-fetch to get the real server data
      await fetchComments();
    } catch {
      // Roll back on failure
      setThreads((prev) => prev.filter((t) => t.id !== optimistic.id));
      throw new Error('Failed to post comment.');
    }
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <CircularProgress size={28} />
      </Box>
    );
  }

  if (error && threads.length === 0) {
    return (
      <Alert severity="error" sx={{ m: 2 }}>
        {error}
      </Alert>
    );
  }

  return (
    <Box>
      {user && (
        <Box sx={{ mb: 2 }}>
          <CommentForm
            onSubmit={handleCreateComment}
            placeholder="Write a comment..."
          />
        </Box>
      )}

      {error && (
        <Alert severity="warning" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      <Divider sx={{ mb: 1 }} />

      {threads.length === 0 ? (
        <Box sx={{ textAlign: 'center', py: 6, color: 'text.secondary' }}>
          <CommentIcon sx={{ fontSize: 40, mb: 1, opacity: 0.4 }} />
          <Typography variant="body2">No comments yet</Typography>
          <Typography variant="caption" color="text.secondary">
            Be the first to start a conversation.
          </Typography>
        </Box>
      ) : (
        <Box>
          {threads.map((thread) => (
            <Box
              key={thread.id}
              sx={{ borderBottom: 1, borderColor: 'divider', '&:last-child': { borderBottom: 0 } }}
            >
              <CommentThread
                thread={thread}
                documentId={documentId}
                currentUserId={user?.id ?? ''}
                currentUserRole={user?.role ?? 'User'}
                onRefresh={fetchComments}
              />
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
};

export default CommentsTab;
