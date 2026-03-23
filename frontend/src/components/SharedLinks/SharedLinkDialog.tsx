import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Typography,
  Alert,
  IconButton,
  InputAdornment,
  CircularProgress,
} from '@mui/material';
import {
  ContentCopy as CopyIcon,
  Check as CheckIcon,
  Visibility as VisibilityIcon,
  VisibilityOff as VisibilityOffIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { sharedLinksService, type CreateSharedLinkRequest, type SharedLinkData } from '../../services/api';
import axios from 'axios';

interface SharedLinkDialogProps {
  open: boolean;
  onClose: () => void;
  documentId: string;
}

const SharedLinkDialog: React.FC<SharedLinkDialogProps> = ({ open, onClose, documentId }) => {
  const { t } = useTranslation();

  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [expiresAt, setExpiresAt] = useState('');
  const [maxViews, setMaxViews] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [createdLink, setCreatedLink] = useState<SharedLinkData | null>(null);
  const [copied, setCopied] = useState(false);

  const resetForm = () => {
    setPassword('');
    setShowPassword(false);
    setExpiresAt('');
    setMaxViews('');
    setLoading(false);
    setError('');
    setCreatedLink(null);
    setCopied(false);
  };

  const handleClose = () => {
    if (!loading) {
      resetForm();
      onClose();
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    setLoading(true);
    setError('');

    const request: CreateSharedLinkRequest = {
      document_id: documentId,
    };

    if (password.trim()) {
      request.password = password.trim();
    }
    if (expiresAt) {
      request.expires_at = new Date(expiresAt).toISOString();
    }
    if (maxViews && parseInt(maxViews, 10) > 0) {
      request.max_views = parseInt(maxViews, 10);
    }

    try {
      const response = await sharedLinksService.create(request);
      setCreatedLink(response.data);
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        setError(err.response.data.error);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Failed to create shared link. Please try again.');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCopyUrl = async () => {
    if (!createdLink) return;

    try {
      await navigator.clipboard.writeText(createdLink.url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setError('Failed to copy to clipboard.');
    }
  };

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      maxWidth="sm"
      fullWidth
      PaperProps={
        createdLink
          ? undefined
          : {
              component: 'form' as React.ElementType,
              onSubmit: handleSubmit,
            }
      }
    >
      <DialogTitle>
        {createdLink ? 'Link Created' : 'Create Shared Link'}
      </DialogTitle>

      <DialogContent>
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {createdLink ? (
          <Box sx={{ mt: 1 }}>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
              Your shared link is ready. Anyone with this link can access the document.
            </Typography>

            <TextField
              fullWidth
              value={createdLink.url}
              slotProps={{
                input: {
                  readOnly: true,
                  endAdornment: (
                    <InputAdornment position="end">
                      <IconButton onClick={handleCopyUrl} edge="end" aria-label="Copy link">
                        {copied ? (
                          <CheckIcon color="success" />
                        ) : (
                          <CopyIcon />
                        )}
                      </IconButton>
                    </InputAdornment>
                  ),
                },
              }}
              sx={{
                '& .MuiInputBase-input': {
                  fontFamily: 'monospace',
                  fontSize: '0.85rem',
                },
              }}
            />

            {copied && (
              <Typography variant="caption" color="success.main" sx={{ mt: 0.5, display: 'block' }}>
                Copied to clipboard
              </Typography>
            )}

            {createdLink.has_password && (
              <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
                This link is password-protected. Recipients will need the password to access the document.
              </Typography>
            )}
          </Box>
        ) : (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2.5, mt: 1 }}>
            <TextField
              label="Password (optional)"
              type={showPassword ? 'text' : 'password'}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              fullWidth
              disabled={loading}
              helperText="Leave empty for unrestricted access"
              slotProps={{
                input: {
                  endAdornment: (
                    <InputAdornment position="end">
                      <IconButton
                        onClick={() => setShowPassword(!showPassword)}
                        edge="end"
                        aria-label={showPassword ? 'Hide password' : 'Show password'}
                      >
                        {showPassword ? <VisibilityOffIcon /> : <VisibilityIcon />}
                      </IconButton>
                    </InputAdornment>
                  ),
                },
              }}
            />

            <TextField
              label="Expiration date (optional)"
              type="datetime-local"
              value={expiresAt}
              onChange={(e) => setExpiresAt(e.target.value)}
              fullWidth
              disabled={loading}
              helperText="Leave empty for no expiration"
              slotProps={{
                inputLabel: { shrink: true },
              }}
            />

            <TextField
              label="Maximum views (optional)"
              type="number"
              value={maxViews}
              onChange={(e) => setMaxViews(e.target.value)}
              fullWidth
              disabled={loading}
              helperText="Leave empty for unlimited views"
              slotProps={{
                htmlInput: { min: 1 },
              }}
            />
          </Box>
        )}
      </DialogContent>

      <DialogActions>
        {createdLink ? (
          <Button onClick={handleClose} variant="contained">
            Done
          </Button>
        ) : (
          <>
            <Button onClick={handleClose} disabled={loading}>
              Cancel
            </Button>
            <Button
              type="submit"
              variant="contained"
              disabled={loading}
              startIcon={loading ? <CircularProgress size={16} /> : undefined}
            >
              {loading ? 'Creating...' : 'Create Link'}
            </Button>
          </>
        )}
      </DialogActions>
    </Dialog>
  );
};

export default SharedLinkDialog;
