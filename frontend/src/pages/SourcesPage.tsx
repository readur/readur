import React, { useState, useEffect } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Button,
  Grid,
  Card,
  CardContent,
  Chip,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Alert,
  LinearProgress,
  Snackbar,
  Divider,
  FormControlLabel,
  Switch,
  Tooltip,
  CircularProgress,
  Fade,
  Stack,
  Avatar,
  Badge,
  useTheme,
  alpha,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
} from '@mui/material';
import {
  Add as AddIcon,
  CloudSync as CloudSyncIcon,
  Error as ErrorIcon,
  CheckCircle as CheckCircleIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  PlayArrow as PlayArrowIcon,
  Storage as StorageIcon,
  Cloud as CloudIcon,
  Speed as SpeedIcon,
  Timeline as TimelineIcon,
  TrendingUp as TrendingUpIcon,
  Security as SecurityIcon,
  AutoFixHigh as AutoFixHighIcon,
  Sync as SyncIcon,
  MoreVert as MoreVertIcon,
  Menu as MenuIcon,
  Folder as FolderIcon,
  Assessment as AssessmentIcon,
  Extension as ExtensionIcon,
  Server as ServerIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import api from '../services/api';
import { formatDistanceToNow } from 'date-fns';

interface Source {
  id: string;
  name: string;
  source_type: 'webdav' | 'local_folder' | 's3';
  enabled: boolean;
  config: any;
  status: 'idle' | 'syncing' | 'error';
  last_sync_at: string | null;
  last_error: string | null;
  last_error_at: string | null;
  total_files_synced: number;
  total_files_pending: number;
  total_size_bytes: number;
  created_at: string;
  updated_at: string;
}

interface SnackbarState {
  open: boolean;
  message: string;
  severity: 'success' | 'error' | 'warning' | 'info';
}

const SourcesPage: React.FC = () => {
  const theme = useTheme();
  const navigate = useNavigate();
  const [sources, setSources] = useState<Source[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<Source | null>(null);
  const [snackbar, setSnackbar] = useState<SnackbarState>({
    open: false,
    message: '',
    severity: 'info',
  });

  // Form state
  const [formData, setFormData] = useState({
    name: '',
    source_type: 'webdav' as 'webdav' | 'local_folder' | 's3',
    enabled: true,
    server_url: '',
    username: '',
    password: '',
    watch_folders: ['/Documents'],
    file_extensions: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
    auto_sync: false,
    sync_interval_minutes: 60,
    server_type: 'generic' as 'nextcloud' | 'owncloud' | 'generic',
  });

  // Additional state for enhanced features
  const [newFolder, setNewFolder] = useState('');
  const [newExtension, setNewExtension] = useState('');
  const [crawlEstimate, setCrawlEstimate] = useState<any>(null);
  const [estimatingCrawl, setEstimatingCrawl] = useState(false);

  const [testingConnection, setTestingConnection] = useState(false);
  const [syncingSource, setSyncingSource] = useState<string | null>(null);

  useEffect(() => {
    loadSources();
  }, []);

  const loadSources = async () => {
    try {
      const response = await api.get('/sources');
      setSources(response.data);
    } catch (error) {
      console.error('Failed to load sources:', error);
      showSnackbar('Failed to load sources', 'error');
    } finally {
      setLoading(false);
    }
  };

  const showSnackbar = (message: string, severity: SnackbarState['severity']) => {
    setSnackbar({ open: true, message, severity });
  };

  const handleCreateSource = () => {
    setEditingSource(null);
    setFormData({
      name: '',
      source_type: 'webdav',
      enabled: true,
      server_url: '',
      username: '',
      password: '',
      watch_folders: ['/Documents'],
      file_extensions: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
      auto_sync: false,
      sync_interval_minutes: 60,
      server_type: 'generic',
    });
    setCrawlEstimate(null);
    setNewFolder('');
    setNewExtension('');
    setDialogOpen(true);
  };

  const handleEditSource = (source: Source) => {
    setEditingSource(source);
    const config = source.config;
    setFormData({
      name: source.name,
      source_type: source.source_type,
      enabled: source.enabled,
      server_url: config.server_url || '',
      username: config.username || '',
      password: config.password || '',
      watch_folders: config.watch_folders || ['/Documents'],
      file_extensions: config.file_extensions || ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
      auto_sync: config.auto_sync || false,
      sync_interval_minutes: config.sync_interval_minutes || 60,
      server_type: config.server_type || 'generic',
    });
    setCrawlEstimate(null);
    setNewFolder('');
    setNewExtension('');
    setDialogOpen(true);
  };

  const handleSaveSource = async () => {
    try {
      const config = {
        server_url: formData.server_url,
        username: formData.username,
        password: formData.password,
        watch_folders: formData.watch_folders,
        file_extensions: formData.file_extensions,
        auto_sync: formData.auto_sync,
        sync_interval_minutes: formData.sync_interval_minutes,
        server_type: formData.server_type,
      };

      if (editingSource) {
        await api.put(`/sources/${editingSource.id}`, {
          name: formData.name,
          enabled: formData.enabled,
          config,
        });
        showSnackbar('Source updated successfully', 'success');
      } else {
        await api.post('/sources', {
          name: formData.name,
          source_type: formData.source_type,
          enabled: formData.enabled,
          config,
        });
        showSnackbar('Source created successfully', 'success');
      }

      setDialogOpen(false);
      loadSources();
    } catch (error) {
      console.error('Failed to save source:', error);
      showSnackbar('Failed to save source', 'error');
    }
  };

  const handleDeleteSource = async (source: Source) => {
    if (!confirm(`Are you sure you want to delete "${source.name}"?`)) {
      return;
    }

    try {
      await api.delete(`/sources/${source.id}`);
      showSnackbar('Source deleted successfully', 'success');
      loadSources();
    } catch (error) {
      console.error('Failed to delete source:', error);
      showSnackbar('Failed to delete source', 'error');
    }
  };

  const handleTestConnection = async () => {
    if (!editingSource) return;

    setTestingConnection(true);
    try {
      const response = await api.post(`/sources/${editingSource.id}/test`);
      if (response.data.success) {
        showSnackbar('Connection successful!', 'success');
      } else {
        showSnackbar(response.data.message || 'Connection failed', 'error');
      }
    } catch (error) {
      console.error('Failed to test connection:', error);
      showSnackbar('Failed to test connection', 'error');
    } finally {
      setTestingConnection(false);
    }
  };

  const handleTriggerSync = async (sourceId: string) => {
    setSyncingSource(sourceId);
    try {
      await api.post(`/sources/${sourceId}/sync`);
      showSnackbar('Sync started successfully', 'success');
      setTimeout(loadSources, 1000);
    } catch (error: any) {
      console.error('Failed to trigger sync:', error);
      if (error.response?.status === 409) {
        showSnackbar('Source is already syncing', 'warning');
      } else {
        showSnackbar('Failed to start sync', 'error');
      }
    } finally {
      setSyncingSource(null);
    }
  };

  // Utility functions for folder management
  const addFolder = () => {
    if (newFolder && !formData.watch_folders.includes(newFolder)) {
      setFormData({
        ...formData,
        watch_folders: [...formData.watch_folders, newFolder]
      });
      setNewFolder('');
    }
  };

  const removeFolder = (folderToRemove: string) => {
    setFormData({
      ...formData,
      watch_folders: formData.watch_folders.filter(folder => folder !== folderToRemove)
    });
  };

  // Utility functions for file extension management
  const addFileExtension = () => {
    if (newExtension && !formData.file_extensions.includes(newExtension)) {
      setFormData({
        ...formData,
        file_extensions: [...formData.file_extensions, newExtension]
      });
      setNewExtension('');
    }
  };

  const removeFileExtension = (extensionToRemove: string) => {
    setFormData({
      ...formData,
      file_extensions: formData.file_extensions.filter(ext => ext !== extensionToRemove)
    });
  };

  // Crawl estimation function
  const estimateCrawl = async () => {
    if (!editingSource) return;

    setEstimatingCrawl(true);
    try {
      const response = await api.post('/webdav/estimate', {
        server_url: formData.server_url,
        username: formData.username,
        password: formData.password,
        watch_folders: formData.watch_folders,
        file_extensions: formData.file_extensions,
        server_type: formData.server_type,
      });
      setCrawlEstimate(response.data);
      showSnackbar('Crawl estimation completed', 'success');
    } catch (error) {
      console.error('Failed to estimate crawl:', error);
      showSnackbar('Failed to estimate crawl', 'error');
    } finally {
      setEstimatingCrawl(false);
    }
  };

  const getSourceIcon = (sourceType: string) => {
    switch (sourceType) {
      case 'webdav':
        return <CloudIcon />;
      case 's3':
        return <StorageIcon />;
      case 'local_folder':
        return <StorageIcon />;
      default:
        return <StorageIcon />;
    }
  };

  const getStatusIcon = (source: Source) => {
    if (source.status === 'syncing') {
      return <SyncIcon sx={{ animation: 'spin 2s linear infinite' }} />;
    } else if (source.status === 'error') {
      return <ErrorIcon />;
    } else {
      return <CheckCircleIcon />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'syncing':
        return theme.palette.info.main;
      case 'error':
        return theme.palette.error.main;
      default:
        return theme.palette.success.main;
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const StatCard = ({ icon, label, value, color = 'primary' }: { 
    icon: React.ReactNode; 
    label: string; 
    value: string | number; 
    color?: 'primary' | 'success' | 'warning' | 'error' 
  }) => (
    <Box 
      sx={{ 
        p: 3, 
        borderRadius: 3,
        background: `linear-gradient(135deg, ${alpha(theme.palette[color].main, 0.1)} 0%, ${alpha(theme.palette[color].main, 0.05)} 100%)`,
        border: `1px solid ${alpha(theme.palette[color].main, 0.2)}`,
        position: 'relative',
        overflow: 'hidden',
        '&::before': {
          content: '""',
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '3px',
          background: `linear-gradient(90deg, ${theme.palette[color].main}, ${theme.palette[color].light})`,
        }
      }}
    >
      <Stack direction="row" alignItems="center" spacing={2}>
        <Avatar 
          sx={{ 
            bgcolor: alpha(theme.palette[color].main, 0.15),
            color: theme.palette[color].main,
            width: 48,
            height: 48,
          }}
        >
          {icon}
        </Avatar>
        <Box>
          <Typography variant="h4" fontWeight="bold" color={theme.palette[color].main}>
            {typeof value === 'number' ? value.toLocaleString() : value}
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {label}
          </Typography>
        </Box>
      </Stack>
    </Box>
  );

  const renderSourceCard = (source: Source) => (
    <Fade in={true} key={source.id}>
      <Card 
        sx={{ 
          position: 'relative',
          overflow: 'hidden',
          borderRadius: 4,
          border: `1px solid ${alpha(theme.palette.divider, 0.1)}`,
          transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
          '&:hover': {
            transform: 'translateY(-4px)',
            boxShadow: theme.shadows[8],
            '& .action-buttons': {
              opacity: 1,
              transform: 'translateY(0)',
            }
          },
          '&::before': {
            content: '""',
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            height: '4px',
            background: source.enabled 
              ? `linear-gradient(90deg, ${getStatusColor(source.status)}, ${alpha(getStatusColor(source.status), 0.7)})`
              : theme.palette.grey[300],
          }
        }}
      >
        <CardContent sx={{ p: 4 }}>
          {/* Header */}
          <Stack direction="row" justifyContent="space-between" alignItems="flex-start" mb={3}>
            <Stack direction="row" alignItems="center" spacing={2}>
              <Avatar 
                sx={{ 
                  bgcolor: alpha(theme.palette.primary.main, 0.1),
                  color: theme.palette.primary.main,
                  width: 56,
                  height: 56,
                }}
              >
                {getSourceIcon(source.source_type)}
              </Avatar>
              <Box>
                <Typography variant="h6" fontWeight="bold" gutterBottom>
                  {source.name}
                </Typography>
                <Stack direction="row" spacing={1} alignItems="center">
                  <Chip
                    label={source.source_type.toUpperCase()}
                    size="small"
                    variant="outlined"
                    sx={{ 
                      borderRadius: 2,
                      fontSize: '0.75rem',
                      fontWeight: 600,
                    }}
                  />
                  <Chip
                    icon={getStatusIcon(source)}
                    label={source.status.charAt(0).toUpperCase() + source.status.slice(1)}
                    size="small"
                    sx={{ 
                      borderRadius: 2,
                      bgcolor: alpha(getStatusColor(source.status), 0.1),
                      color: getStatusColor(source.status),
                      border: `1px solid ${alpha(getStatusColor(source.status), 0.3)}`,
                      fontSize: '0.75rem',
                      fontWeight: 600,
                    }}
                  />
                  {!source.enabled && (
                    <Chip 
                      label="Disabled" 
                      size="small" 
                      color="default"
                      sx={{ borderRadius: 2, fontSize: '0.75rem' }}
                    />
                  )}
                </Stack>
              </Box>
            </Stack>

            {/* Action Buttons */}
            <Stack 
              direction="row" 
              spacing={1}
              className="action-buttons"
              sx={{
                opacity: 0,
                transform: 'translateY(-8px)',
                transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
              }}
            >
              <Tooltip title="Trigger Sync">
                <span>
                  <IconButton
                    onClick={() => handleTriggerSync(source.id)}
                    disabled={source.status === 'syncing' || !source.enabled}
                    sx={{
                      bgcolor: alpha(theme.palette.primary.main, 0.1),
                      '&:hover': { bgcolor: alpha(theme.palette.primary.main, 0.2) },
                    }}
                  >
                    {syncingSource === source.id ? (
                      <CircularProgress size={20} />
                    ) : (
                      <PlayArrowIcon />
                    )}
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title="Edit Source">
                <IconButton 
                  onClick={() => handleEditSource(source)}
                  sx={{
                    bgcolor: alpha(theme.palette.grey[500], 0.1),
                    '&:hover': { bgcolor: alpha(theme.palette.grey[500], 0.2) },
                  }}
                >
                  <EditIcon />
                </IconButton>
              </Tooltip>
              <Tooltip title="Delete Source">
                <IconButton 
                  onClick={() => handleDeleteSource(source)}
                  sx={{
                    bgcolor: alpha(theme.palette.error.main, 0.1),
                    '&:hover': { bgcolor: alpha(theme.palette.error.main, 0.2) },
                    color: theme.palette.error.main,
                  }}
                >
                  <DeleteIcon />
                </IconButton>
              </Tooltip>
            </Stack>
          </Stack>

          {/* Stats Grid */}
          <Grid container spacing={3} mb={3}>
            <Grid item xs={6} sm={3}>
              <StatCard
                icon={<TrendingUpIcon />}
                label="Files Synced"
                value={source.total_files_synced}
                color="success"
              />
            </Grid>
            <Grid item xs={6} sm={3}>
              <StatCard
                icon={<SpeedIcon />}
                label="Files Pending"
                value={source.total_files_pending}
                color="warning"
              />
            </Grid>
            <Grid item xs={6} sm={3}>
              <StatCard
                icon={<StorageIcon />}
                label="Total Size"
                value={formatBytes(source.total_size_bytes)}
                color="primary"
              />
            </Grid>
            <Grid item xs={6} sm={3}>
              <StatCard
                icon={<TimelineIcon />}
                label="Last Sync"
                value={source.last_sync_at
                  ? formatDistanceToNow(new Date(source.last_sync_at), { addSuffix: true })
                  : 'Never'}
                color="primary"
              />
            </Grid>
          </Grid>

          {/* Error Alert */}
          {source.last_error && (
            <Alert 
              severity="error" 
              sx={{ 
                borderRadius: 3,
                '& .MuiAlert-icon': {
                  fontSize: '1.2rem',
                }
              }}
            >
              <Typography variant="body2" fontWeight="medium">
                {source.last_error}
              </Typography>
              {source.last_error_at && (
                <Typography variant="caption" display="block" sx={{ mt: 0.5, opacity: 0.8 }}>
                  {formatDistanceToNow(new Date(source.last_error_at), { addSuffix: true })}
                </Typography>
              )}
            </Alert>
          )}
        </CardContent>
      </Card>
    </Fade>
  );

  return (
    <Container maxWidth="xl" sx={{ py: 6 }}>
      {/* Header */}
      <Box sx={{ mb: 6 }}>
        <Typography 
          variant="h3" 
          component="h1" 
          fontWeight="bold"
          sx={{
            background: `linear-gradient(45deg, ${theme.palette.primary.main}, ${theme.palette.secondary.main})`,
            backgroundClip: 'text',
            WebkitBackgroundClip: 'text',
            color: 'transparent',
            mb: 2,
          }}
        >
          Document Sources
        </Typography>
        <Typography variant="h6" color="text.secondary" sx={{ mb: 4 }}>
          Connect and manage your document sources with intelligent syncing
        </Typography>
        
        <Stack direction="row" spacing={2} alignItems="center">
          <Button
            variant="contained"
            size="large"
            startIcon={<AddIcon />}
            onClick={handleCreateSource}
            sx={{
              borderRadius: 3,
              px: 4,
              py: 1.5,
              background: `linear-gradient(45deg, ${theme.palette.primary.main}, ${theme.palette.primary.dark})`,
              boxShadow: `0 8px 32px ${alpha(theme.palette.primary.main, 0.3)}`,
              '&:hover': {
                transform: 'translateY(-2px)',
                boxShadow: `0 12px 40px ${alpha(theme.palette.primary.main, 0.4)}`,
              },
              transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            }}
          >
            Add Source
          </Button>
          
          <Button
            variant="outlined"
            size="large"
            startIcon={<AutoFixHighIcon />}
            onClick={loadSources}
            sx={{
              borderRadius: 3,
              px: 4,
              py: 1.5,
              borderWidth: 2,
              '&:hover': {
                borderWidth: 2,
                transform: 'translateY(-1px)',
              },
              transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            }}
          >
            Refresh
          </Button>
        </Stack>
      </Box>

      {/* Content */}
      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }}>
          <CircularProgress size={48} thickness={3} />
        </Box>
      ) : sources.length === 0 ? (
        <Paper 
          sx={{ 
            p: 8, 
            textAlign: 'center',
            borderRadius: 4,
            background: `linear-gradient(135deg, ${alpha(theme.palette.primary.main, 0.05)} 0%, ${alpha(theme.palette.secondary.main, 0.05)} 100%)`,
            border: `1px solid ${alpha(theme.palette.divider, 0.1)}`,
          }}
        >
          <Avatar 
            sx={{ 
              width: 80, 
              height: 80, 
              mx: 'auto', 
              mb: 3,
              bgcolor: alpha(theme.palette.primary.main, 0.1),
              color: theme.palette.primary.main,
            }}
          >
            <StorageIcon sx={{ fontSize: 40 }} />
          </Avatar>
          <Typography variant="h5" fontWeight="bold" gutterBottom>
            No Sources Configured
          </Typography>
          <Typography variant="body1" color="text.secondary" sx={{ mb: 4, maxWidth: 400, mx: 'auto' }}>
            Connect your first document source to start automatically syncing and processing your files with AI-powered OCR.
          </Typography>
          <Button
            variant="contained"
            size="large"
            startIcon={<AddIcon />}
            onClick={handleCreateSource}
            sx={{
              borderRadius: 3,
              px: 6,
              py: 2,
              fontSize: '1.1rem',
              background: `linear-gradient(45deg, ${theme.palette.primary.main}, ${theme.palette.primary.dark})`,
              boxShadow: `0 8px 32px ${alpha(theme.palette.primary.main, 0.3)}`,
              '&:hover': {
                transform: 'translateY(-2px)',
                boxShadow: `0 12px 40px ${alpha(theme.palette.primary.main, 0.4)}`,
              },
              transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            }}
          >
            Add Your First Source
          </Button>
        </Paper>
      ) : (
        <Grid container spacing={4}>
          {sources.map(renderSourceCard)}
        </Grid>
      )}

      {/* Create/Edit Dialog - Enhanced */}
      <Dialog 
        open={dialogOpen} 
        onClose={() => setDialogOpen(false)} 
        maxWidth="md" 
        fullWidth
        PaperProps={{
          sx: {
            borderRadius: 4,
            background: theme.palette.background.paper,
          }
        }}
      >
        <DialogTitle sx={{ p: 4, pb: 2 }}>
          <Stack direction="row" alignItems="center" spacing={2}>
            <Avatar 
              sx={{ 
                bgcolor: alpha(theme.palette.primary.main, 0.1),
                color: theme.palette.primary.main,
              }}
            >
              {editingSource ? <EditIcon /> : <AddIcon />}
            </Avatar>
            <Box>
              <Typography variant="h6" fontWeight="bold">
                {editingSource ? 'Edit Source' : 'Create New Source'}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                {editingSource ? 'Update your source configuration' : 'Connect a new document source'}
              </Typography>
            </Box>
          </Stack>
        </DialogTitle>
        
        <DialogContent sx={{ p: 4, pt: 2 }}>
          <Stack spacing={3}>
            <TextField
              fullWidth
              label="Source Name"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="My Document Server"
              sx={{
                '& .MuiOutlinedInput-root': {
                  borderRadius: 2,
                }
              }}
            />

            {!editingSource && (
              <FormControl fullWidth>
                <InputLabel>Source Type</InputLabel>
                <Select
                  value={formData.source_type}
                  onChange={(e) => setFormData({ ...formData, source_type: e.target.value as any })}
                  label="Source Type"
                  sx={{
                    borderRadius: 2,
                  }}
                >
                  <MenuItem value="webdav">
                    <Stack direction="row" alignItems="center" spacing={2}>
                      <CloudIcon />
                      <Box>
                        <Typography variant="body1">WebDAV</Typography>
                        <Typography variant="caption" color="text.secondary">
                          Nextcloud, ownCloud, and other WebDAV servers
                        </Typography>
                      </Box>
                    </Stack>
                  </MenuItem>
                  <MenuItem value="local_folder" disabled>
                    <Stack direction="row" alignItems="center" spacing={2}>
                      <StorageIcon />
                      <Box>
                        <Typography variant="body1">Local Folder</Typography>
                        <Typography variant="caption" color="text.secondary">
                          Coming Soon
                        </Typography>
                      </Box>
                    </Stack>
                  </MenuItem>
                  <MenuItem value="s3" disabled>
                    <Stack direction="row" alignItems="center" spacing={2}>
                      <CloudIcon />
                      <Box>
                        <Typography variant="body1">S3 Compatible</Typography>
                        <Typography variant="caption" color="text.secondary">
                          Coming Soon
                        </Typography>
                      </Box>
                    </Stack>
                  </MenuItem>
                </Select>
              </FormControl>
            )}

            {formData.source_type === 'webdav' && (
              <Paper 
                sx={{ 
                  p: 3, 
                  borderRadius: 3,
                  bgcolor: alpha(theme.palette.primary.main, 0.03),
                  border: `1px solid ${alpha(theme.palette.primary.main, 0.1)}`,
                }}
              >
                <Stack spacing={3}>
                  <Stack direction="row" alignItems="center" spacing={2} mb={2}>
                    <Avatar 
                      sx={{ 
                        bgcolor: alpha(theme.palette.primary.main, 0.1),
                        color: theme.palette.primary.main,
                        width: 32,
                        height: 32,
                      }}
                    >
                      <CloudIcon />
                    </Avatar>
                    <Typography variant="h6" fontWeight="medium">
                      WebDAV Configuration
                    </Typography>
                  </Stack>
                  
                  <TextField
                    fullWidth
                    label="Server URL"
                    value={formData.server_url}
                    onChange={(e) => setFormData({ ...formData, server_url: e.target.value })}
                    placeholder="https://nextcloud.example.com/remote.php/dav/files/username/"
                    sx={{ '& .MuiOutlinedInput-root': { borderRadius: 2 } }}
                  />

                  <Grid container spacing={2}>
                    <Grid item xs={12} sm={6}>
                      <TextField
                        fullWidth
                        label="Username"
                        value={formData.username}
                        onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                        sx={{ '& .MuiOutlinedInput-root': { borderRadius: 2 } }}
                      />
                    </Grid>
                    <Grid item xs={12} sm={6}>
                      <TextField
                        fullWidth
                        label="Password"
                        type="password"
                        value={formData.password}
                        onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                        sx={{ '& .MuiOutlinedInput-root': { borderRadius: 2 } }}
                      />
                    </Grid>
                  </Grid>

                  <FormControl fullWidth>
                    <InputLabel>Server Type</InputLabel>
                    <Select
                      value={formData.server_type}
                      onChange={(e) => setFormData({ ...formData, server_type: e.target.value as any })}
                      label="Server Type"
                      sx={{ borderRadius: 2 }}
                    >
                      <MenuItem value="nextcloud">
                        <Stack direction="row" alignItems="center" spacing={2}>
                          <ServerIcon />
                          <Box>
                            <Typography variant="body1">Nextcloud</Typography>
                            <Typography variant="caption" color="text.secondary">
                              Optimized for Nextcloud servers
                            </Typography>
                          </Box>
                        </Stack>
                      </MenuItem>
                      <MenuItem value="owncloud">
                        <Stack direction="row" alignItems="center" spacing={2}>
                          <ServerIcon />
                          <Box>
                            <Typography variant="body1">ownCloud</Typography>
                            <Typography variant="caption" color="text.secondary">
                              Optimized for ownCloud servers
                            </Typography>
                          </Box>
                        </Stack>
                      </MenuItem>
                      <MenuItem value="generic">
                        <Stack direction="row" alignItems="center" spacing={2}>
                          <CloudIcon />
                          <Box>
                            <Typography variant="body1">Generic WebDAV</Typography>
                            <Typography variant="caption" color="text.secondary">
                              Standard WebDAV protocol
                            </Typography>
                          </Box>
                        </Stack>
                      </MenuItem>
                    </Select>
                  </FormControl>

                  <FormControlLabel
                    control={
                      <Switch
                        checked={formData.auto_sync}
                        onChange={(e) => setFormData({ ...formData, auto_sync: e.target.checked })}
                      />
                    }
                    label={
                      <Box>
                        <Typography variant="body2" fontWeight="medium">
                          Enable Automatic Sync
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          Automatically sync files on a schedule
                        </Typography>
                      </Box>
                    }
                  />

                  {formData.auto_sync && (
                    <TextField
                      fullWidth
                      type="number"
                      label="Sync Interval (minutes)"
                      value={formData.sync_interval_minutes}
                      onChange={(e) => setFormData({ ...formData, sync_interval_minutes: parseInt(e.target.value) || 60 })}
                      inputProps={{ min: 15, max: 1440 }}
                      helperText="How often to check for new files (15 min - 24 hours)"
                      sx={{ '& .MuiOutlinedInput-root': { borderRadius: 2 } }}
                    />
                  )}

                  <Divider sx={{ my: 2 }} />

                  {/* Folder Management */}
                  <Stack direction="row" alignItems="center" spacing={2} mb={2}>
                    <Avatar 
                      sx={{ 
                        bgcolor: alpha(theme.palette.secondary.main, 0.1),
                        color: theme.palette.secondary.main,
                        width: 32,
                        height: 32,
                      }}
                    >
                      <FolderIcon />
                    </Avatar>
                    <Typography variant="h6" fontWeight="medium">
                      Folders to Monitor
                    </Typography>
                  </Stack>
                  
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    Specify which folders to scan for files. Use absolute paths starting with "/".
                  </Typography>

                  <Stack direction="row" spacing={1} mb={2}>
                    <TextField
                      label="Add Folder Path"
                      value={newFolder}
                      onChange={(e) => setNewFolder(e.target.value)}
                      placeholder="/Documents"
                      sx={{ 
                        flexGrow: 1,
                        '& .MuiOutlinedInput-root': { borderRadius: 2 } 
                      }}
                    />
                    <Button 
                      variant="outlined" 
                      onClick={addFolder} 
                      disabled={!newFolder}
                      sx={{ borderRadius: 2, px: 3 }}
                    >
                      Add
                    </Button>
                  </Stack>

                  <Box sx={{ mb: 3 }}>
                    {formData.watch_folders.map((folder, index) => (
                      <Chip
                        key={index}
                        label={folder}
                        onDelete={() => removeFolder(folder)}
                        sx={{ 
                          mr: 1, 
                          mb: 1,
                          borderRadius: 2,
                          bgcolor: alpha(theme.palette.secondary.main, 0.1),
                          color: theme.palette.secondary.main,
                        }}
                      />
                    ))}
                  </Box>

                  {/* File Extensions */}
                  <Stack direction="row" alignItems="center" spacing={2} mb={2}>
                    <Avatar 
                      sx={{ 
                        bgcolor: alpha(theme.palette.warning.main, 0.1),
                        color: theme.palette.warning.main,
                        width: 32,
                        height: 32,
                      }}
                    >
                      <ExtensionIcon />
                    </Avatar>
                    <Typography variant="h6" fontWeight="medium">
                      File Extensions
                    </Typography>
                  </Stack>
                  
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    File types to sync and process with OCR.
                  </Typography>

                  <Stack direction="row" spacing={1} mb={2}>
                    <TextField
                      label="Add Extension"
                      value={newExtension}
                      onChange={(e) => setNewExtension(e.target.value)}
                      placeholder="docx"
                      sx={{ 
                        flexGrow: 1,
                        '& .MuiOutlinedInput-root': { borderRadius: 2 } 
                      }}
                    />
                    <Button 
                      variant="outlined" 
                      onClick={addFileExtension} 
                      disabled={!newExtension}
                      sx={{ borderRadius: 2, px: 3 }}
                    >
                      Add
                    </Button>
                  </Stack>

                  <Box sx={{ mb: 3 }}>
                    {formData.file_extensions.map((extension, index) => (
                      <Chip
                        key={index}
                        label={extension}
                        onDelete={() => removeFileExtension(extension)}
                        sx={{ 
                          mr: 1, 
                          mb: 1,
                          borderRadius: 2,
                          bgcolor: alpha(theme.palette.warning.main, 0.1),
                          color: theme.palette.warning.main,
                        }}
                      />
                    ))}
                  </Box>

                  {/* Crawl Estimation */}
                  {editingSource && formData.server_url && formData.username && formData.watch_folders.length > 0 && (
                    <>
                      <Divider sx={{ my: 2 }} />
                      
                      <Stack direction="row" alignItems="center" spacing={2} mb={2}>
                        <Avatar 
                          sx={{ 
                            bgcolor: alpha(theme.palette.info.main, 0.1),
                            color: theme.palette.info.main,
                            width: 32,
                            height: 32,
                          }}
                        >
                          <AssessmentIcon />
                        </Avatar>
                        <Typography variant="h6" fontWeight="medium">
                          Crawl Estimation
                        </Typography>
                      </Stack>
                      
                      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                        Estimate how many files will be processed and how long it will take.
                      </Typography>

                      <Button
                        variant="outlined"
                        onClick={estimateCrawl}
                        disabled={estimatingCrawl}
                        startIcon={estimatingCrawl ? <CircularProgress size={20} /> : <AssessmentIcon />}
                        sx={{ mb: 2, borderRadius: 2 }}
                      >
                        {estimatingCrawl ? 'Estimating...' : 'Estimate Crawl'}
                      </Button>

                      {estimatingCrawl && (
                        <Box sx={{ mb: 2 }}>
                          <LinearProgress sx={{ borderRadius: 1 }} />
                          <Typography variant="body2" sx={{ mt: 1 }}>
                            Analyzing folders and counting files...
                          </Typography>
                        </Box>
                      )}

                      {crawlEstimate && (
                        <Paper 
                          sx={{ 
                            p: 3, 
                            borderRadius: 3,
                            bgcolor: alpha(theme.palette.info.main, 0.03),
                            border: `1px solid ${alpha(theme.palette.info.main, 0.1)}`,
                            mb: 2
                          }}
                        >
                          <Typography variant="h6" sx={{ mb: 2 }}>
                            Estimation Results
                          </Typography>
                          <Grid container spacing={2} sx={{ mb: 2 }}>
                            <Grid item xs={6} sm={3}>
                              <Box sx={{ textAlign: 'center' }}>
                                <Typography variant="h4" color="primary">
                                  {crawlEstimate.total_files?.toLocaleString() || '0'}
                                </Typography>
                                <Typography variant="body2">Total Files</Typography>
                              </Box>
                            </Grid>
                            <Grid item xs={6} sm={3}>
                              <Box sx={{ textAlign: 'center' }}>
                                <Typography variant="h4" color="success.main">
                                  {crawlEstimate.total_supported_files?.toLocaleString() || '0'}
                                </Typography>
                                <Typography variant="body2">Supported Files</Typography>
                              </Box>
                            </Grid>
                            <Grid item xs={6} sm={3}>
                              <Box sx={{ textAlign: 'center' }}>
                                <Typography variant="h4" color="warning.main">
                                  {crawlEstimate.total_estimated_time_hours?.toFixed(1) || '0'}h
                                </Typography>
                                <Typography variant="body2">Estimated Time</Typography>
                              </Box>
                            </Grid>
                            <Grid item xs={6} sm={3}>
                              <Box sx={{ textAlign: 'center' }}>
                                <Typography variant="h4" color="info.main">
                                  {crawlEstimate.total_size_mb ? (crawlEstimate.total_size_mb / 1024).toFixed(1) : '0'}GB
                                </Typography>
                                <Typography variant="body2">Total Size</Typography>
                              </Box>
                            </Grid>
                          </Grid>

                          {crawlEstimate.folders && crawlEstimate.folders.length > 0 && (
                            <TableContainer component={Paper} sx={{ borderRadius: 2 }}>
                              <Table size="small">
                                <TableHead>
                                  <TableRow>
                                    <TableCell>Folder</TableCell>
                                    <TableCell align="right">Total Files</TableCell>
                                    <TableCell align="right">Supported</TableCell>
                                    <TableCell align="right">Est. Time</TableCell>
                                    <TableCell align="right">Size (MB)</TableCell>
                                  </TableRow>
                                </TableHead>
                                <TableBody>
                                  {crawlEstimate.folders.map((folder: any) => (
                                    <TableRow key={folder.path}>
                                      <TableCell>{folder.path}</TableCell>
                                      <TableCell align="right">{folder.total_files?.toLocaleString() || '0'}</TableCell>
                                      <TableCell align="right">{folder.supported_files?.toLocaleString() || '0'}</TableCell>
                                      <TableCell align="right">{folder.estimated_time_hours?.toFixed(1) || '0'}h</TableCell>
                                      <TableCell align="right">{folder.total_size_mb?.toFixed(1) || '0'}</TableCell>
                                    </TableRow>
                                  ))}
                                </TableBody>
                              </Table>
                            </TableContainer>
                          )}
                        </Paper>
                      )}
                    </>
                  )}
                </Stack>
              </Paper>
            )}

            <FormControlLabel
              control={
                <Switch
                  checked={formData.enabled}
                  onChange={(e) => setFormData({ ...formData, enabled: e.target.checked })}
                />
              }
              label={
                <Box>
                  <Typography variant="body2" fontWeight="medium">
                    Source Enabled
                  </Typography>
                  <Typography variant="caption" color="text.secondary">
                    Enable this source for syncing
                  </Typography>
                </Box>
              }
            />
          </Stack>
        </DialogContent>

        <DialogActions sx={{ p: 4, pt: 2 }}>
          <Button 
            onClick={() => setDialogOpen(false)}
            sx={{ borderRadius: 2 }}
          >
            Cancel
          </Button>
          {editingSource && formData.source_type === 'webdav' && (
            <Button
              onClick={handleTestConnection}
              disabled={testingConnection}
              startIcon={testingConnection ? <CircularProgress size={20} /> : <SecurityIcon />}
              sx={{ borderRadius: 2 }}
            >
              Test Connection
            </Button>
          )}
          <Button 
            onClick={handleSaveSource} 
            variant="contained"
            sx={{
              borderRadius: 2,
              px: 4,
              background: `linear-gradient(45deg, ${theme.palette.primary.main}, ${theme.palette.primary.dark})`,
            }}
          >
            {editingSource ? 'Save Changes' : 'Create Source'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Snackbar */}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      >
        <Alert
          onClose={() => setSnackbar({ ...snackbar, open: false })}
          severity={snackbar.severity}
          sx={{ 
            width: '100%',
            borderRadius: 3,
          }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>

      {/* Custom CSS for animations */}
      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </Container>
  );
};

export default SourcesPage;