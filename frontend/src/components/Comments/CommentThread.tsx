import React, { useState } from 'react';
import { Box, Button, Collapse, Typography } from '@mui/material';
import { ExpandMore as ExpandMoreIcon, ExpandLess as ExpandLessIcon } from '@mui/icons-material';
import { type CommentThread as CommentThreadType } from '../../services/api';
import { commentsService } from '../../services/api';
import CommentItem from './CommentItem';
import CommentForm from './CommentForm';

interface CommentThreadProps {
  thread: CommentThreadType;
  documentId: string;
  currentUserId: string;
  currentUserRole: string;
  onRefresh: () => void;
}

const CommentThread: React.FC<CommentThreadProps> = ({
  thread,
  documentId,
  currentUserId,
  currentUserRole,
  onRefresh,
}) => {
  const [repliesExpanded, setRepliesExpanded] = useState(false);
  const [showReplyForm, setShowReplyForm] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [extraReplies, setExtraReplies] = useState<typeof thread.replies>([]);

  const allReplies = [...thread.replies, ...extraReplies];
  const hasMoreReplies = thread.reply_count > allReplies.length;

  const handleReply = async (content: string) => {
    await commentsService.create(documentId, { content, parent_id: thread.id });
    setShowReplyForm(false);
    setRepliesExpanded(true);
    onRefresh();
  };

  const handleEdit = async (commentId: string, content: string) => {
    await commentsService.update(documentId, commentId, { content });
    onRefresh();
  };

  const handleDelete = async (commentId: string) => {
    await commentsService.delete(documentId, commentId);
    onRefresh();
  };

  const handleLoadMore = async () => {
    setLoadingMore(true);
    try {
      const response = await commentsService.getReplies(
        documentId,
        thread.id,
        50,
        allReplies.length
      );
      setExtraReplies((prev) => [...prev, ...response.data]);
    } finally {
      setLoadingMore(false);
    }
  };

  return (
    <Box>
      <CommentItem
        comment={thread}
        onReply={() => setShowReplyForm((prev) => !prev)}
        onEdit={handleEdit}
        onDelete={handleDelete}
        currentUserId={currentUserId}
        currentUserRole={currentUserRole}
      />

      {showReplyForm && (
        <Box sx={{ ml: 5.5, mt: 0.5, mb: 1 }}>
          <CommentForm
            onSubmit={handleReply}
            placeholder="Write a reply..."
            autoFocus
            onCancel={() => setShowReplyForm(false)}
          />
        </Box>
      )}

      {thread.reply_count > 0 && (
        <Box sx={{ ml: 5.5 }}>
          <Button
            size="small"
            onClick={() => setRepliesExpanded((prev) => !prev)}
            startIcon={repliesExpanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
            sx={{ textTransform: 'none', color: 'text.secondary', mb: 0.5 }}
          >
            {repliesExpanded ? 'Hide' : 'Show'} {thread.reply_count} {thread.reply_count === 1 ? 'reply' : 'replies'}
          </Button>

          <Collapse in={repliesExpanded}>
            <Box sx={{ borderLeft: 2, borderColor: 'divider', pl: 2 }}>
              {allReplies.map((reply) => (
                <CommentItem
                  key={reply.id}
                  comment={reply}
                  onEdit={handleEdit}
                  onDelete={handleDelete}
                  currentUserId={currentUserId}
                  currentUserRole={currentUserRole}
                />
              ))}

              {hasMoreReplies && (
                <Button
                  size="small"
                  onClick={handleLoadMore}
                  disabled={loadingMore}
                  sx={{ textTransform: 'none', mt: 0.5 }}
                >
                  {loadingMore ? 'Loading...' : `Load more replies`}
                </Button>
              )}
            </Box>
          </Collapse>
        </Box>
      )}
    </Box>
  );
};

export default CommentThread;
