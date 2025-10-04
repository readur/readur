import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Button,
  Chip,
  IconButton,
  TextField,
  InputAdornment,
  Stack,
  CircularProgress,
  Alert,
  Pagination,
  FormControl,
  InputLabel,
  Select,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Checkbox,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Tooltip,
  MenuItem,
  Breadcrumbs,
  Link,
} from '@mui/material';
import {
  Search as SearchIcon,
  FilterList as FilterIcon,
  Delete as DeleteIcon,
  Block as BlockIcon,
  Refresh as RefreshIcon,
  Folder as FolderIcon,
  Cloud as CloudIcon,
  Computer as ComputerIcon,
  Storage as StorageIcon,
  CalendarToday as DateIcon,
  ArrowBack as ArrowBackIcon,
  RestoreFromTrash as RestoreFromTrashIcon,
} from '@mui/icons-material';
import { format, formatDistanceToNow } from 'date-fns';
import { useNotifications } from '../contexts/NotificationContext';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

interface IgnoredFile {
  id: string;
  file_hash: string;
  filename: string;
  original_filename: string;
  file_path: string;
  file_size: number;
  mime_type: string;
  source_type?: string;
  source_path?: string;
  source_identifier?: string;
  ignored_at: string;
  ignored_by: string;
  ignored_by_username?: string;
  reason?: string;
  created_at: string;
}

interface IgnoredFilesStats {
  total_ignored_files: number;
  by_source_type: Array<{
    source_type?: string;
    count: number;
    total_size_bytes: number;
  }>;
  total_size_bytes: number;
  most_recent_ignored_at?: string;
}

const IgnoredFilesPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [ignoredFiles, setIgnoredFiles] = useState<IgnoredFile[]>([]);
  const [stats, setStats] = useState<IgnoredFilesStats | null>(null);
  const [sources, setSources] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [searchTerm, setSearchTerm] = useState('');
  const [sourceTypeFilter, setSourceTypeFilter] = useState('');
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [bulkDeleteDialog, setBulkDeleteDialog] = useState(false);
  const [deletingFiles, setDeletingFiles] = useState(false);
  const { addNotification } = useNotifications();

  // URL parameters for filtering
  const sourceTypeParam = searchParams.get('sourceType');
  const sourceNameParam = searchParams.get('sourceName');
  const sourceIdParam = searchParams.get('sourceId');

  const pageSize = 25;

  const fetchIgnoredFiles = async () => {
    setLoading(true);
    setError(null);
    
    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const params = new URLSearchParams({
        limit: pageSize.toString(),
        offset: ((page - 1) * pageSize).toString(),
      });

      if (searchTerm) {
        params.append('filename', searchTerm);
      }

      if (sourceTypeFilter) {
        params.append('source_type', sourceTypeFilter);
      }

      if (sourceIdParam) {
        params.append('source_identifier', sourceIdParam);
      }

      const response = await fetch(`/api/ignored-files?${params}`, {
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch ignored files: ${response.statusText}`);
      }

      const data = await response.json();
      setIgnoredFiles(data.ignored_files);
      setTotalPages(Math.ceil(data.total / pageSize));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load ignored files');
      console.error('Error fetching ignored files:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchStats = async () => {
    try {
      const token = localStorage.getItem('token');
      if (!token) return;

      const response = await fetch('/api/ignored-files/stats', {
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (response.ok) {
        const data = await response.json();
        setStats(data);
      }
    } catch (err) {
      console.error('Error fetching stats:', err);
    }
  };

  useEffect(() => {
    // Set initial filters from URL params
    if (sourceTypeParam) {
      setSourceTypeFilter(sourceTypeParam);
    }
    fetchSources();
  }, []);

  useEffect(() => {
    fetchIgnoredFiles();
    fetchStats();
  }, [page, searchTerm, sourceTypeFilter]);

  const fetchSources = async () => {
    try {
      const token = localStorage.getItem('token');
      if (!token) return;

      const response = await fetch('/api/sources', {
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (response.ok) {
        const data = await response.json();
        setSources(data);
      }
    } catch (err) {
      console.error('Error fetching sources:', err);
    }
  };

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(event.target.value);
    setPage(1);
  };

  const handleSourceTypeFilter = (event: any) => {
    setSourceTypeFilter(event.target.value);
    setPage(1);
  };

  const handleSelectFile = (fileId: string) => {
    const newSelected = new Set(selectedFiles);
    if (newSelected.has(fileId)) {
      newSelected.delete(fileId);
    } else {
      newSelected.add(fileId);
    }
    setSelectedFiles(newSelected);
  };

  const handleSelectAll = () => {
    if (selectedFiles.size === ignoredFiles.length) {
      setSelectedFiles(new Set());
    } else {
      setSelectedFiles(new Set(ignoredFiles.map(file => file.id)));
    }
  };

  const handleDeleteSelected = async () => {
    if (selectedFiles.size === 0) return;

    setDeletingFiles(true);
    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch('/api/ignored-files/bulk-delete', {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          ignored_file_ids: Array.from(selectedFiles)
        }),
      });

      if (!response.ok) {
        throw new Error(`Failed to delete ignored files: ${response.statusText}`);
      }

      const data = await response.json();
      addNotification({
        type: 'success',
        title: 'Files Deleted',
        message: data.message
      });
      setSelectedFiles(new Set());
      setBulkDeleteDialog(false);
      fetchIgnoredFiles();
      fetchStats();
    } catch (err) {
      addNotification({
        type: 'error',
        title: 'Delete Failed',
        message: err instanceof Error ? err.message : 'Failed to delete ignored files'
      });
    } finally {
      setDeletingFiles(false);
    }
  };

  const handleDeleteSingle = async (fileId: string) => {
    try {
      const token = localStorage.getItem('token');
      if (!token) {
        throw new Error('No authentication token found');
      }

      const response = await fetch(`/api/ignored-files/${fileId}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to delete ignored file: ${response.statusText}`);
      }

      const data = await response.json();
      addNotification({
        type: 'success',
        title: 'Files Deleted',
        message: data.message
      });
      fetchIgnoredFiles();
      fetchStats();
    } catch (err) {
      addNotification({
        type: 'error',
        title: 'Delete Failed',
        message: err instanceof Error ? err.message : 'Failed to delete ignored file'
      });
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const getSourceIcon = (sourceType?: string) => {
    switch (sourceType) {
      case 'webdav':
        return <CloudIcon fontSize="small" />;
      case 'local_folder':
        return <ComputerIcon fontSize="small" />;
      case 's3':
        return <StorageIcon fontSize="small" />;
      default:
        return <FolderIcon fontSize="small" />;
    }
  };

  const getSourceTypeDisplay = (sourceType?: string) => {
    switch (sourceType) {
      case 'webdav':
        return 'WebDAV';
      case 'local_folder':
        return 'Local Folder';
      case 's3':
        return 'S3';
      default:
        return sourceType || 'Unknown';
    }
  };

  const getSourceNameFromIdentifier = (sourceIdentifier?: string, sourceType?: string) => {
    // Try to find the source name from the sources list
    const source = sources.find(s => s.id === sourceIdentifier || s.name.toLowerCase().includes(sourceIdentifier?.toLowerCase() || ''));
    return source?.name || sourceIdentifier || 'Unknown Source';
  };

  const clearFilters = () => {
    setSourceTypeFilter('');
    setSearchTerm('');
    setPage(1);
    navigate('/ignored-files', { replace: true });
  };

  const uniqueSourceTypes = Array.from(
    new Set(ignoredFiles.map(file => file.source_type).filter(Boolean))
  );

  return (
    <Box sx={{ p: 3 }}>
      {/* Breadcrumbs and Navigation */}
      <Box sx={{ mb: 3 }}>
        <Breadcrumbs aria-label="breadcrumb" sx={{ mb: 2 }}>
          <Link
            color="inherit"
            href="#"
            onClick={() => navigate('/sources')}
            sx={{ display: 'flex', alignItems: 'center', textDecoration: 'none' }}
          >
            <StorageIcon sx={{ mr: 0.5 }} fontSize="inherit" />
            {t('navigation.sources')}
          </Link>
          <Typography color="text.primary" sx={{ display: 'flex', alignItems: 'center' }}>
            <BlockIcon sx={{ mr: 0.5 }} fontSize="inherit" />
            {t('ignoredFiles.title')}
            {(sourceTypeParam || sourceNameParam) && (
              <Chip
                label={sourceNameParam ? `${sourceNameParam}` : `${getSourceTypeDisplay(sourceTypeParam)} Sources`}
                size="small"
                onDelete={clearFilters}
                sx={{ ml: 1 }}
              />
            )}
          </Typography>
        </Breadcrumbs>
      </Box>

      <Typography variant="h4" gutterBottom sx={{ fontWeight: 'bold' }}>
        <BlockIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
        {t('ignoredFiles.title')}
        {(sourceTypeParam || sourceNameParam || sourceIdParam) && (
          <Button
            variant="outlined"
            size="small"
            startIcon={<ArrowBackIcon />}
            onClick={clearFilters}
            sx={{ ml: 2, textTransform: 'none' }}
          >
            {t('common.actions.viewDetails')}
          </Button>
        )}
      </Typography>

      <Typography variant="body1" color="text.secondary" sx={{ mb: 3 }}>
        {t('ignoredFiles.subtitle')}
      </Typography>

      {/* Statistics Cards */}
      {stats && (
        <Box sx={{ mb: 3 }}>
          <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
            <Card variant="outlined">
              <CardContent>
                <Typography variant="h6" color="primary">
                  {stats.total_ignored_files}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Ignored Files
                </Typography>
              </CardContent>
            </Card>
            <Card variant="outlined">
              <CardContent>
                <Typography variant="h6" color="primary">
                  {formatFileSize(stats.total_size_bytes)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Size
                </Typography>
              </CardContent>
            </Card>
            {stats.most_recent_ignored_at && (
              <Card variant="outlined">
                <CardContent>
                  <Typography variant="h6" color="primary">
                    {formatDistanceToNow(new Date(stats.most_recent_ignored_at), { addSuffix: true })}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    Most Recent
                  </Typography>
                </CardContent>
              </Card>
            )}
          </Stack>
        </Box>
      )}

      {/* Filters and Search */}
      <Card variant="outlined" sx={{ mb: 3 }}>
        <CardContent>
          <Stack direction={{ xs: 'column', md: 'row' }} spacing={2} alignItems="center">
            <TextField
              placeholder={t('ignoredFiles.filters.searchPlaceholder')}
              variant="outlined"
              size="small"
              value={searchTerm}
              onChange={handleSearch}
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <SearchIcon />
                  </InputAdornment>
                ),
              }}
              sx={{ flexGrow: 1 }}
            />

            <FormControl size="small" sx={{ minWidth: 150 }}>
              <InputLabel>{t('ignoredFiles.filters.reason')}</InputLabel>
              <Select
                value={sourceTypeFilter}
                label={t('ignoredFiles.filters.reason')}
                onChange={handleSourceTypeFilter}
              >
                <MenuItem value="">{t('ignoredFiles.filters.allReasons')}</MenuItem>
                {uniqueSourceTypes.map(sourceType => (
                  <MenuItem key={sourceType} value={sourceType}>
                    {getSourceTypeDisplay(sourceType)}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <Button
              variant="outlined"
              startIcon={<RefreshIcon />}
              onClick={() => {
                fetchIgnoredFiles();
                fetchStats();
              }}
            >
              {t('common.actions.refresh')}
            </Button>
          </Stack>
        </CardContent>
      </Card>

      {/* Bulk Actions */}
      {selectedFiles.size > 0 && (
        <Card variant="outlined" sx={{ mb: 2, bgcolor: 'action.selected' }}>
          <CardContent>
            <Stack direction="row" spacing={2} alignItems="center">
              <Typography variant="body2">
                {selectedFiles.size} file{selectedFiles.size !== 1 ? 's' : ''} selected
              </Typography>
              <Button
                variant="contained"
                color="success"
                startIcon={<RestoreFromTrashIcon />}
                onClick={() => setBulkDeleteDialog(true)}
                size="small"
              >
                Remove from Ignored List
              </Button>
            </Stack>
          </CardContent>
        </Card>
      )}

      {/* Files Table */}
      <Card variant="outlined">
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell padding="checkbox">
                  <Checkbox
                    indeterminate={selectedFiles.size > 0 && selectedFiles.size < ignoredFiles.length}
                    checked={ignoredFiles.length > 0 && selectedFiles.size === ignoredFiles.length}
                    onChange={handleSelectAll}
                  />
                </TableCell>
                <TableCell>{t('ignoredFiles.table.filename')}</TableCell>
                <TableCell>Source</TableCell>
                <TableCell>{t('ignoredFiles.table.size')}</TableCell>
                <TableCell>{t('ignoredFiles.table.ignoredAt')}</TableCell>
                <TableCell>{t('ignoredFiles.table.reason')}</TableCell>
                <TableCell>{t('common.actions.edit')}</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={7} align="center">
                    <CircularProgress />
                  </TableCell>
                </TableRow>
              ) : error ? (
                <TableRow>
                  <TableCell colSpan={7}>
                    <Alert severity="error">{error}</Alert>
                  </TableCell>
                </TableRow>
              ) : ignoredFiles.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} align="center">
                    <Typography variant="body2" color="text.secondary">
                      {t('ignoredFiles.empty.subtitle')}
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                ignoredFiles.map((file) => (
                  <TableRow key={file.id} hover>
                    <TableCell padding="checkbox">
                      <Checkbox
                        checked={selectedFiles.has(file.id)}
                        onChange={() => handleSelectFile(file.id)}
                      />
                    </TableCell>
                    <TableCell>
                      <Box>
                        <Typography variant="body2" fontWeight="medium">
                          {file.filename}
                        </Typography>
                        {file.filename !== file.original_filename && (
                          <Typography variant="caption" color="text.secondary">
                            Original: {file.original_filename}
                          </Typography>
                        )}
                        <Typography variant="caption" color="text.secondary" display="block">
                          {file.mime_type}
                        </Typography>
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Stack direction="row" spacing={1} alignItems="center">
                        {getSourceIcon(file.source_type)}
                        <Box>
                          <Typography variant="body2">
                            {getSourceTypeDisplay(file.source_type)}
                          </Typography>
                          {file.source_path && (
                            <Typography variant="caption" color="text.secondary">
                              {file.source_path}
                            </Typography>
                          )}
                        </Box>
                      </Stack>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {formatFileSize(file.file_size)}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {format(new Date(file.ignored_at), 'MMM dd, yyyy')}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {formatDistanceToNow(new Date(file.ignored_at), { addSuffix: true })}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {file.reason || 'No reason provided'}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Stack direction="row" spacing={1}>
                        <Tooltip title="Remove from ignored list (allow re-syncing)">
                          <IconButton
                            size="small"
                            onClick={() => handleDeleteSingle(file.id)}
                            color="success"
                          >
                            <RestoreFromTrashIcon fontSize="small" />
                          </IconButton>
                        </Tooltip>
                      </Stack>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>

        {/* Pagination */}
        {totalPages > 1 && (
          <Box sx={{ p: 2, display: 'flex', justifyContent: 'center' }}>
            <Pagination
              count={totalPages}
              page={page}
              onChange={(_, newPage) => setPage(newPage)}
              color="primary"
            />
          </Box>
        )}
      </Card>

      {/* Bulk Delete Confirmation Dialog */}
      <Dialog open={bulkDeleteDialog} onClose={() => setBulkDeleteDialog(false)}>
        <DialogTitle>Confirm Bulk Delete</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to remove {selectedFiles.size} file{selectedFiles.size !== 1 ? 's' : ''} from the ignored list?
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            These files will be eligible for syncing again if encountered from their sources. This action allows them to be re-imported during future syncs.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setBulkDeleteDialog(false)}>Cancel</Button>
          <Button
            onClick={handleDeleteSelected}
            color="success"
            variant="contained"
            disabled={deletingFiles}
            startIcon={deletingFiles ? <CircularProgress size={16} /> : <RestoreFromTrashIcon />}
          >
            Remove from Ignored List
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default IgnoredFilesPage;