import React, { useState, useEffect, useCallback } from 'react';
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material';
import {
  Add as AddIcon,
  Check as CheckIcon,
  ContentCopy as CopyIcon,
  Delete as DeleteIcon,
  Key as KeyIcon,
  Warning as WarningIcon,
} from '@mui/icons-material';
import axios from 'axios';
import { apiKeysService, type ApiKey } from '../../services/api';

type ExpirationOption = '7' | '30' | '90' | '180' | '365' | 'never';

const EXPIRATION_OPTIONS: { value: ExpirationOption; label: string }[] = [
  { value: '7', label: '7 days' },
  { value: '30', label: '30 days' },
  { value: '90', label: '90 days' },
  { value: '180', label: '180 days' },
  { value: '365', label: '1 year' },
  { value: 'never', label: 'Never (not recommended)' },
];

function formatDate(value: string | null): string {
  if (!value) return '—';
  return new Date(value).toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function statusChip(key: ApiKey) {
  if (key.revoked_at) return <Chip label="Revoked" size="small" color="default" />;
  if (key.is_expired) return <Chip label="Expired" size="small" color="warning" />;
  return <Chip label="Active" size="small" color="success" />;
}

function extractErrorMessage(err: unknown, fallback: string): string {
  if (axios.isAxiosError(err) && err.response?.data?.error) {
    return String(err.response.data.error);
  }
  return fallback;
}

const ApiKeysManager: React.FC = () => {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [createOpen, setCreateOpen] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyExpiration, setNewKeyExpiration] = useState<ExpirationOption>('90');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // After a successful create, hold the plaintext so the user can copy it.
  // This is the ONLY time the plaintext exists client-side — it is cleared as
  // soon as the reveal dialog is closed and never sent anywhere else.
  const [revealedKey, setRevealedKey] = useState<{ name: string; plaintext: string } | null>(null);
  const [copied, setCopied] = useState(false);

  const [revokeTarget, setRevokeTarget] = useState<ApiKey | null>(null);
  const [revoking, setRevoking] = useState(false);
  const [revokeError, setRevokeError] = useState<string | null>(null);

  const fetchKeys = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await apiKeysService.list();
      setKeys(response.data);
    } catch (err) {
      setError(extractErrorMessage(err, 'Failed to load API keys.'));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchKeys();
  }, [fetchKeys]);

  const resetCreateForm = () => {
    setNewKeyName('');
    setNewKeyExpiration('90');
    setCreateError(null);
  };

  const handleOpenCreate = () => {
    resetCreateForm();
    setCreateOpen(true);
  };

  const handleCloseCreate = () => {
    if (creating) return;
    setCreateOpen(false);
    resetCreateForm();
  };

  const handleCreate = async () => {
    const name = newKeyName.trim();
    if (!name) {
      setCreateError('Please give this key a name.');
      return;
    }
    setCreating(true);
    setCreateError(null);
    try {
      const expiresInDays =
        newKeyExpiration === 'never' ? null : Number(newKeyExpiration);
      const response = await apiKeysService.create({
        name,
        expires_in_days: expiresInDays ?? undefined,
      });
      setCreateOpen(false);
      setRevealedKey({
        name: response.data.api_key.name,
        plaintext: response.data.plaintext,
      });
      resetCreateForm();
      await fetchKeys();
    } catch (err) {
      setCreateError(extractErrorMessage(err, 'Failed to create API key.'));
    } finally {
      setCreating(false);
    }
  };

  const handleCopyPlaintext = async () => {
    if (!revealedKey) return;
    try {
      await navigator.clipboard.writeText(revealedKey.plaintext);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setError('Failed to copy to clipboard. Please select and copy manually.');
    }
  };

  const handleCloseReveal = () => {
    setRevealedKey(null);
    setCopied(false);
  };

  const handleRevoke = async () => {
    if (!revokeTarget) return;
    setRevoking(true);
    setRevokeError(null);
    try {
      await apiKeysService.revoke(revokeTarget.id);
      setRevokeTarget(null);
      await fetchKeys();
    } catch (err) {
      // Surface the error inside the revoke dialog rather than on the page
      // behind it, where a modal overlay would hide it from the user.
      setRevokeError(extractErrorMessage(err, 'Failed to revoke API key.'));
    } finally {
      setRevoking(false);
    }
  };

  return (
    <Box>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 2 }}>
        <Box>
          <Typography variant="h6">API Keys</Typography>
          <Typography variant="body2" color="text.secondary">
            Use an API key in the <code>Authorization: Bearer</code> header to authenticate
            scripts and integrations. Keys carry your full account permissions — treat them
            like passwords.
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<AddIcon />}
          onClick={handleOpenCreate}
        >
          Create API Key
        </Button>
      </Stack>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress size={28} />
        </Box>
      ) : keys.length === 0 ? (
        <Box
          sx={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            py: 6,
            color: 'text.secondary',
          }}
        >
          <KeyIcon sx={{ fontSize: 40, mb: 1, opacity: 0.4 }} />
          <Typography variant="body2">You don't have any API keys yet.</Typography>
        </Box>
      ) : (
        <TableContainer component={Paper} variant="outlined">
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Name</TableCell>
                <TableCell>Prefix</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Last used</TableCell>
                <TableCell>Expires</TableCell>
                <TableCell>Created</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {keys.map((key) => (
                <TableRow key={key.id} hover>
                  <TableCell>{key.name}</TableCell>
                  <TableCell>
                    <Typography
                      variant="body2"
                      sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}
                    >
                      {key.key_prefix}…
                    </Typography>
                  </TableCell>
                  <TableCell>{statusChip(key)}</TableCell>
                  <TableCell>
                    <Typography variant="body2" color="text.secondary">
                      {formatDate(key.last_used_at)}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" color="text.secondary">
                      {key.expires_at ? formatDate(key.expires_at) : 'Never'}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" color="text.secondary">
                      {formatDate(key.created_at)}
                    </Typography>
                  </TableCell>
                  <TableCell align="right">
                    <Tooltip title={key.revoked_at ? 'Already revoked' : 'Revoke'}>
                      <span>
                        <IconButton
                          size="small"
                          color="error"
                          disabled={!!key.revoked_at}
                          onClick={() => {
                            setRevokeError(null);
                            setRevokeTarget(key);
                          }}
                          aria-label="Revoke API key"
                        >
                          <DeleteIcon fontSize="small" />
                        </IconButton>
                      </span>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}

      {/* Create dialog */}
      <Dialog open={createOpen} onClose={handleCloseCreate} maxWidth="xs" fullWidth>
        <DialogTitle>Create API Key</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <TextField
              label="Name"
              placeholder="e.g. backup-script"
              value={newKeyName}
              onChange={(e) => setNewKeyName(e.target.value)}
              fullWidth
              autoFocus
              inputProps={{ maxLength: 100 }}
              helperText="A label you'll recognize later."
            />
            <FormControl fullWidth>
              <InputLabel id="api-key-expiration-label">Expiration</InputLabel>
              <Select
                labelId="api-key-expiration-label"
                label="Expiration"
                value={newKeyExpiration}
                onChange={(e) => setNewKeyExpiration(e.target.value as ExpirationOption)}
              >
                {EXPIRATION_OPTIONS.map((opt) => (
                  <MenuItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
            {createError && <Alert severity="error">{createError}</Alert>}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseCreate} disabled={creating}>
            Cancel
          </Button>
          <Button
            onClick={handleCreate}
            variant="contained"
            disabled={creating}
            startIcon={creating ? <CircularProgress size={16} /> : undefined}
          >
            {creating ? 'Creating…' : 'Create'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* One-time reveal dialog */}
      <Dialog
        open={!!revealedKey}
        onClose={(_, reason) => {
          // Require an explicit click on "I've saved it" — backdrop clicks
          // must not dismiss this dialog, since that is the last chance to copy.
          if (reason !== 'backdropClick') handleCloseReveal();
        }}
        maxWidth="sm"
        fullWidth
        disableEscapeKeyDown
      >
        <DialogTitle>Your new API key</DialogTitle>
        <DialogContent>
          <Alert severity="warning" icon={<WarningIcon />} sx={{ mb: 2 }}>
            Copy this key now. For your security it will not be shown again —
            if you lose it you'll need to create a new one.
          </Alert>
          {revealedKey && (
            <>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                Key name: <strong>{revealedKey.name}</strong>
              </Typography>
              <Paper
                variant="outlined"
                sx={{
                  p: 1.5,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 1,
                  fontFamily: 'monospace',
                  fontSize: '0.85rem',
                  wordBreak: 'break-all',
                }}
              >
                <Box sx={{ flex: 1 }}>{revealedKey.plaintext}</Box>
                <Tooltip title={copied ? 'Copied!' : 'Copy to clipboard'}>
                  <IconButton
                    size="small"
                    onClick={handleCopyPlaintext}
                    aria-label="Copy API key"
                  >
                    {copied ? (
                      <CheckIcon fontSize="small" color="success" />
                    ) : (
                      <CopyIcon fontSize="small" />
                    )}
                  </IconButton>
                </Tooltip>
              </Paper>
              <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
                Use it like:
                {' '}
                <code>curl -H "Authorization: Bearer {revealedKey.plaintext.slice(0, 15)}…"</code>
              </Typography>
            </>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseReveal} variant="contained">
            I've saved it
          </Button>
        </DialogActions>
      </Dialog>

      {/* Revoke confirmation */}
      <Dialog
        open={!!revokeTarget}
        onClose={() => {
          if (revoking) return;
          setRevokeTarget(null);
          setRevokeError(null);
        }}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Revoke API key</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Revoke <strong>{revokeTarget?.name}</strong>? Any scripts or integrations using
            this key will immediately stop working. This cannot be undone.
          </DialogContentText>
          {revokeError && (
            <Alert severity="error" sx={{ mt: 2 }} onClose={() => setRevokeError(null)}>
              {revokeError}
            </Alert>
          )}
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => {
              setRevokeTarget(null);
              setRevokeError(null);
            }}
            disabled={revoking}
          >
            Cancel
          </Button>
          <Button
            onClick={handleRevoke}
            color="error"
            variant="contained"
            disabled={revoking}
            startIcon={revoking ? <CircularProgress size={16} /> : undefined}
          >
            {revoking ? 'Revoking…' : 'Revoke'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default ApiKeysManager;
