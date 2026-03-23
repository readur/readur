import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Typography,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  IconButton,
  Tooltip,
  CircularProgress,
  Alert,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogContentText,
  DialogActions,
  Button,
} from '@mui/material';
import {
  Delete as DeleteIcon,
  ContentCopy as CopyIcon,
  Check as CheckIcon,
  LinkOff as LinkOffIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { sharedLinksService, type SharedLinkData } from '../../services/api';
import axios from 'axios';

interface SharedLinksManagerProps {
  documentId: string;
}

function getStatusChip(link: SharedLinkData) {
  if (link.is_revoked) {
    return <Chip label="Revoked" size="small" color="default" />;
  }
  if (link.is_expired) {
    return <Chip label="Expired" size="small" color="warning" />;
  }
  return <Chip label="Active" size="small" color="success" />;
}

function truncateToken(token: string): string {
  if (token.length <= 12) return token;
  return `${token.slice(0, 6)}...${token.slice(-4)}`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return 'Never';
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

const SharedLinksManager: React.FC<SharedLinksManagerProps> = ({ documentId }) => {
  const { t } = useTranslation();

  const [links, setLinks] = useState<SharedLinkData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [revokeTarget, setRevokeTarget] = useState<SharedLinkData | null>(null);
  const [revoking, setRevoking] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const fetchLinks = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const response = await sharedLinksService.listByDocument(documentId);
      setLinks(response.data);
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Failed to load shared links.');
      }
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    fetchLinks();
  }, [fetchLinks]);

  const handleRevoke = async () => {
    if (!revokeTarget) return;

    setRevoking(true);
    try {
      await sharedLinksService.revoke(revokeTarget.id);
      setRevokeTarget(null);
      await fetchLinks();
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        setError(err.response.data.error);
      } else {
        setError('Failed to revoke shared link.');
      }
    } finally {
      setRevoking(false);
    }
  };

  const handleCopyUrl = async (link: SharedLinkData) => {
    try {
      await navigator.clipboard.writeText(link.url);
      setCopiedId(link.id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      setError('Failed to copy to clipboard.');
    }
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
        <CircularProgress size={28} />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ mb: 2 }}>
        {error}
      </Alert>
    );
  }

  if (links.length === 0) {
    return (
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          py: 4,
          color: 'text.secondary',
        }}
      >
        <LinkOffIcon sx={{ fontSize: 40, mb: 1, opacity: 0.4 }} />
        <Typography variant="body2">No shared links yet</Typography>
      </Box>
    );
  }

  return (
    <>
      <TableContainer component={Paper} variant="outlined">
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>Link</TableCell>
              <TableCell>Status</TableCell>
              <TableCell align="right">Views</TableCell>
              <TableCell>Expires</TableCell>
              <TableCell>Created</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {links.map((link) => (
              <TableRow key={link.id} hover>
                <TableCell>
                  <Tooltip title={link.token}>
                    <Typography
                      variant="body2"
                      sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}
                    >
                      {truncateToken(link.token)}
                    </Typography>
                  </Tooltip>
                </TableCell>
                <TableCell>{getStatusChip(link)}</TableCell>
                <TableCell align="right">
                  <Typography variant="body2">
                    {link.view_count}
                    {link.max_views != null ? ` / ${link.max_views}` : ''}
                  </Typography>
                </TableCell>
                <TableCell>
                  <Typography variant="body2" color="text.secondary">
                    {formatDate(link.expires_at)}
                  </Typography>
                </TableCell>
                <TableCell>
                  <Typography variant="body2" color="text.secondary">
                    {formatDate(link.created_at)}
                  </Typography>
                </TableCell>
                <TableCell align="right">
                  <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 0.5 }}>
                    <Tooltip title="Copy link">
                      <IconButton
                        size="small"
                        onClick={() => handleCopyUrl(link)}
                        disabled={link.is_revoked || link.is_expired}
                        aria-label="Copy link URL"
                      >
                        {copiedId === link.id ? (
                          <CheckIcon fontSize="small" color="success" />
                        ) : (
                          <CopyIcon fontSize="small" />
                        )}
                      </IconButton>
                    </Tooltip>
                    <Tooltip title="Revoke link">
                      <span>
                        <IconButton
                          size="small"
                          onClick={() => setRevokeTarget(link)}
                          disabled={link.is_revoked}
                          color="error"
                          aria-label="Revoke shared link"
                        >
                          <DeleteIcon fontSize="small" />
                        </IconButton>
                      </span>
                    </Tooltip>
                  </Box>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>

      <Dialog
        open={!!revokeTarget}
        onClose={() => !revoking && setRevokeTarget(null)}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Revoke Shared Link</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to revoke this shared link? Anyone with this link will no longer be able to access the document.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setRevokeTarget(null)} disabled={revoking}>
            Cancel
          </Button>
          <Button
            onClick={handleRevoke}
            color="error"
            variant="contained"
            disabled={revoking}
            startIcon={revoking ? <CircularProgress size={16} /> : undefined}
          >
            {revoking ? 'Revoking...' : 'Revoke'}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

export default SharedLinksManager;
