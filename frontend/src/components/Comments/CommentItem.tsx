import React, { useState } from 'react';
import {
  Box,
  Avatar,
  Typography,
  IconButton,
  Tooltip,
  Stack,
} from '@mui/material';
import {
  Reply as ReplyIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
} from '@mui/icons-material';
import { type CommentWithAuthor } from '../../services/api';
import CommentForm from './CommentForm';

interface CommentItemProps {
  comment: CommentWithAuthor;
  onReply?: () => void;
  onEdit: (commentId: string, content: string) => Promise<void>;
  onDelete: (commentId: string) => Promise<void>;
  currentUserId: string;
  currentUserRole: string;
}

function formatRelativeTime(dateStr: string): string {
  const now = Date.now();
  const date = new Date(dateStr).getTime();
  const diffSeconds = Math.floor((now - date) / 1000);

  if (diffSeconds < 60) return 'just now';
  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return `${diffMinutes} minute${diffMinutes !== 1 ? 's' : ''} ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours} hour${diffHours !== 1 ? 's' : ''} ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 30) return `${diffDays} day${diffDays !== 1 ? 's' : ''} ago`;
  const diffMonths = Math.floor(diffDays / 30);
  if (diffMonths < 12) return `${diffMonths} month${diffMonths !== 1 ? 's' : ''} ago`;
  const diffYears = Math.floor(diffMonths / 12);
  return `${diffYears} year${diffYears !== 1 ? 's' : ''} ago`;
}

const CommentItem: React.FC<CommentItemProps> = ({
  comment,
  onReply,
  onEdit,
  onDelete,
  currentUserId,
  currentUserRole,
}) => {
  const [editing, setEditing] = useState(false);
  const isAuthor = currentUserId === comment.user_id;
  const isAdmin = currentUserRole === 'Admin';
  const canEdit = isAuthor;
  const canDelete = isAuthor || isAdmin;

  const handleEdit = async (content: string) => {
    await onEdit(comment.id, content);
    setEditing(false);
  };

  return (
    <Box
      sx={{
        display: 'flex',
        gap: 1.5,
        py: 1.5,
        '&:hover .comment-actions': { opacity: 1 },
      }}
    >
      <Avatar
        sx={{
          width: 32,
          height: 32,
          fontSize: 14,
          fontWeight: 600,
          bgcolor: 'primary.main',
          flexShrink: 0,
        }}
      >
        {comment.username.charAt(0).toUpperCase()}
      </Avatar>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 0.25 }}>
          <Typography variant="subtitle2" sx={{ fontWeight: 600, lineHeight: 1.3 }}>
            {comment.username}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {formatRelativeTime(comment.created_at)}
          </Typography>
          {comment.is_edited && (
            <Typography variant="caption" color="text.secondary" sx={{ fontStyle: 'italic' }}>
              (edited)
            </Typography>
          )}
        </Stack>

        {editing ? (
          <CommentForm
            onSubmit={handleEdit}
            initialValue={comment.content}
            submitLabel="Save"
            onCancel={() => setEditing(false)}
            autoFocus
          />
        ) : (
          <Typography
            variant="body2"
            sx={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', lineHeight: 1.5 }}
          >
            {comment.content}
          </Typography>
        )}

        {!editing && (
          <Stack
            direction="row"
            spacing={0.5}
            className="comment-actions"
            sx={{
              mt: 0.5,
              opacity: { xs: 1, md: 0 },
              transition: 'opacity 0.15s ease',
            }}
          >
            {onReply && (
              <Tooltip title="Reply">
                <IconButton size="small" onClick={onReply} aria-label="Reply">
                  <ReplyIcon sx={{ fontSize: 16 }} />
                </IconButton>
              </Tooltip>
            )}
            {canEdit && (
              <Tooltip title="Edit">
                <IconButton size="small" onClick={() => setEditing(true)} aria-label="Edit">
                  <EditIcon sx={{ fontSize: 16 }} />
                </IconButton>
              </Tooltip>
            )}
            {canDelete && (
              <Tooltip title="Delete">
                <IconButton size="small" onClick={() => onDelete(comment.id)} aria-label="Delete">
                  <DeleteIcon sx={{ fontSize: 16 }} />
                </IconButton>
              </Tooltip>
            )}
          </Stack>
        )}
      </Box>
    </Box>
  );
};

export default CommentItem;
