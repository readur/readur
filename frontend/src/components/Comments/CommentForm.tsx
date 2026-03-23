import React, { useState } from 'react';
import { Box, TextField, Button, CircularProgress } from '@mui/material';
import { Send as SendIcon } from '@mui/icons-material';

interface CommentFormProps {
  onSubmit: (content: string) => Promise<void>;
  placeholder?: string;
  autoFocus?: boolean;
  initialValue?: string;
  submitLabel?: string;
  onCancel?: () => void;
}

const CommentForm: React.FC<CommentFormProps> = ({
  onSubmit,
  placeholder = 'Write a comment...',
  autoFocus = false,
  initialValue = '',
  submitLabel = 'Submit',
  onCancel,
}) => {
  const [content, setContent] = useState(initialValue);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = content.trim();
    if (!trimmed || submitting) return;

    setSubmitting(true);
    try {
      await onSubmit(trimmed);
      setContent('');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Box component="form" onSubmit={handleSubmit} sx={{ display: 'flex', gap: 1, alignItems: 'flex-end' }}>
      <TextField
        value={content}
        onChange={(e) => setContent(e.target.value)}
        placeholder={placeholder}
        autoFocus={autoFocus}
        multiline
        minRows={2}
        maxRows={4}
        fullWidth
        size="small"
        disabled={submitting}
        sx={{ flex: 1 }}
      />
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
        <Button
          type="submit"
          variant="contained"
          size="small"
          disabled={!content.trim() || submitting}
          startIcon={submitting ? <CircularProgress size={16} /> : <SendIcon />}
          sx={{ textTransform: 'none', whiteSpace: 'nowrap' }}
        >
          {submitLabel}
        </Button>
        {onCancel && (
          <Button
            size="small"
            onClick={onCancel}
            disabled={submitting}
            sx={{ textTransform: 'none' }}
          >
            Cancel
          </Button>
        )}
      </Box>
    </Box>
  );
};

export default CommentForm;
