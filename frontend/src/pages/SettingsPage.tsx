import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Tabs,
  Tab,
  FormControl,
  FormControlLabel,
  InputLabel,
  Select,
  MenuItem,
  Button,
  Snackbar,
  Alert,
  TextField,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Grid,
  Card,
  CardContent,
  Divider,
  Switch,
  SelectChangeEvent,
  Chip,
  LinearProgress,
  CircularProgress,
} from '@mui/material';
import { Edit as EditIcon, Delete as DeleteIcon, Add as AddIcon, 
         CloudSync as CloudSyncIcon, Folder as FolderIcon,
         Assessment as AssessmentIcon, PlayArrow as PlayArrowIcon } from '@mui/icons-material';
import { useAuth } from '../contexts/AuthContext';
import api from '../services/api';

interface User {
  id: string;
  username: string;
  email: string;
  created_at: string;
}

interface Settings {
  ocrLanguage: string;
  concurrentOcrJobs: number;
  ocrTimeoutSeconds: number;
  maxFileSizeMb: number;
  allowedFileTypes: string[];
  autoRotateImages: boolean;
  enableImagePreprocessing: boolean;
  searchResultsPerPage: number;
  searchSnippetLength: number;
  fuzzySearchThreshold: number;
  retentionDays: number | null;
  enableAutoCleanup: boolean;
  enableCompression: boolean;
  memoryLimitMb: number;
  cpuPriority: string;
  enableBackgroundOcr: boolean;
  webdavEnabled: boolean;
  webdavServerUrl: string;
  webdavUsername: string;
  webdavPassword: string;
  webdavWatchFolders: string[];
  webdavFileExtensions: string[];
  webdavAutoSync: boolean;
  webdavSyncIntervalMinutes: number;
}

interface SnackbarState {
  open: boolean;
  message: string;
  severity: 'success' | 'error' | 'warning' | 'info';
}

interface UserDialogState {
  open: boolean;
  mode: 'create' | 'edit';
  user: User | null;
}

interface UserFormData {
  username: string;
  email: string;
  password: string;
}

interface OcrLanguage {
  code: string;
  name: string;
}

interface WebDAVFolderInfo {
  path: string;
  total_files: number;
  supported_files: number;
  estimated_time_hours: number;
  total_size_mb: number;
}

interface WebDAVCrawlEstimate {
  folders: WebDAVFolderInfo[];
  total_files: number;
  total_supported_files: number;
  total_estimated_time_hours: number;
  total_size_mb: number;
}

interface WebDAVConnectionResult {
  success: boolean;
  message: string;
  server_version?: string;
  server_type?: string;
}

interface WebDAVTabContentProps {
  settings: Settings;
  loading: boolean;
  onSettingsChange: (key: keyof Settings, value: any) => Promise<void>;
  onShowSnackbar: (message: string, severity: 'success' | 'error' | 'warning' | 'info') => void;
}

// Debounce utility function
function useDebounce<T extends (...args: any[]) => any>(func: T, delay: number): T {
  const timeoutRef = React.useRef<NodeJS.Timeout | null>(null);

  const debouncedFunc = useCallback((...args: Parameters<T>) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    timeoutRef.current = setTimeout(() => func(...args), delay);
  }, [func, delay]) as T;

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return debouncedFunc;
}

const WebDAVTabContent: React.FC<WebDAVTabContentProps> = ({ 
  settings, 
  loading, 
  onSettingsChange, 
  onShowSnackbar 
}) => {
  const [connectionResult, setConnectionResult] = useState<WebDAVConnectionResult | null>(null);
  const [testingConnection, setTestingConnection] = useState(false);
  const [crawlEstimate, setCrawlEstimate] = useState<WebDAVCrawlEstimate | null>(null);
  const [estimatingCrawl, setEstimatingCrawl] = useState(false);
  const [newFolder, setNewFolder] = useState('');
  
  // WebDAV sync state
  const [syncStatus, setSyncStatus] = useState<any>(null);
  const [startingSync, setStartingSync] = useState(false);
  const [cancellingSync, setCancellingSync] = useState(false);
  const [pollingSyncStatus, setPollingSyncStatus] = useState(false);
  
  // Local state for input fields to prevent focus loss
  const [localWebdavServerUrl, setLocalWebdavServerUrl] = useState(settings.webdavServerUrl);
  const [localWebdavUsername, setLocalWebdavUsername] = useState(settings.webdavUsername);
  const [localWebdavPassword, setLocalWebdavPassword] = useState(settings.webdavPassword);
  const [localSyncInterval, setLocalSyncInterval] = useState(settings.webdavSyncIntervalMinutes);

  // Update local state when settings change from outside (like initial load)
  useEffect(() => {
    setLocalWebdavServerUrl(settings.webdavServerUrl);
    setLocalWebdavUsername(settings.webdavUsername);
    setLocalWebdavPassword(settings.webdavPassword);
    setLocalSyncInterval(settings.webdavSyncIntervalMinutes);
  }, [settings.webdavServerUrl, settings.webdavUsername, settings.webdavPassword, settings.webdavSyncIntervalMinutes]);

  // Debounced update functions
  const debouncedUpdateServerUrl = useDebounce((value: string) => {
    onSettingsChange('webdavServerUrl', value);
  }, 500);

  const debouncedUpdateUsername = useDebounce((value: string) => {
    onSettingsChange('webdavUsername', value);
  }, 500);

  const debouncedUpdatePassword = useDebounce((value: string) => {
    onSettingsChange('webdavPassword', value);
  }, 500);

  const debouncedUpdateSyncInterval = useDebounce((value: number) => {
    onSettingsChange('webdavSyncIntervalMinutes', value);
  }, 500);

  // Input change handlers
  const handleServerUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setLocalWebdavServerUrl(value);
    debouncedUpdateServerUrl(value);
  };

  const handleUsernameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setLocalWebdavUsername(value);
    debouncedUpdateUsername(value);
  };

  const handlePasswordChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setLocalWebdavPassword(value);
    debouncedUpdatePassword(value);
  };

  const handleSyncIntervalChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(e.target.value);
    setLocalSyncInterval(value);
    debouncedUpdateSyncInterval(value);
  };

  const testConnection = async () => {
    if (!localWebdavServerUrl || !localWebdavUsername || !localWebdavPassword) {
      onShowSnackbar('Please fill in all WebDAV connection details', 'warning');
      return;
    }

    setTestingConnection(true);
    try {
      const response = await api.post('/webdav/test-connection', {
        server_url: localWebdavServerUrl,
        username: localWebdavUsername,
        password: localWebdavPassword,
        server_type: 'nextcloud'
      });
      setConnectionResult(response.data);
      onShowSnackbar(response.data.message, response.data.success ? 'success' : 'error');
    } catch (error: any) {
      console.error('Connection test failed:', error);
      setConnectionResult({
        success: false,
        message: 'Connection test failed'
      });
      onShowSnackbar('Connection test failed', 'error');
    } finally {
      setTestingConnection(false);
    }
  };

  const estimateCrawl = async () => {
    if (!settings.webdavEnabled || settings.webdavWatchFolders.length === 0) {
      onShowSnackbar('Please enable WebDAV and configure folders first', 'warning');
      return;
    }

    setEstimatingCrawl(true);
    try {
      const response = await api.post('/webdav/estimate-crawl', {
        folders: settings.webdavWatchFolders
      });
      setCrawlEstimate(response.data);
      onShowSnackbar('Crawl estimation completed', 'success');
    } catch (error: any) {
      console.error('Crawl estimation failed:', error);
      onShowSnackbar('Failed to estimate crawl', 'error');
    } finally {
      setEstimatingCrawl(false);
    }
  };

  const addFolder = () => {
    if (newFolder && !settings.webdavWatchFolders.includes(newFolder)) {
      onSettingsChange('webdavWatchFolders', [...settings.webdavWatchFolders, newFolder]);
      setNewFolder('');
    }
  };

  const removeFolder = (folderToRemove: string) => {
    onSettingsChange('webdavWatchFolders', settings.webdavWatchFolders.filter(f => f !== folderToRemove));
  };

  const serverTypes = [
    { value: 'nextcloud', label: 'Nextcloud' },
    { value: 'owncloud', label: 'ownCloud' },
    { value: 'generic', label: 'Generic WebDAV' },
  ];

  // WebDAV sync functions
  const fetchSyncStatus = async () => {
    try {
      const response = await api.get('/webdav/sync-status');
      setSyncStatus(response.data);
    } catch (error) {
      console.error('Failed to fetch sync status:', error);
    }
  };

  const startManualSync = async () => {
    setStartingSync(true);
    try {
      const response = await api.post('/webdav/start-sync');
      if (response.data.success) {
        onShowSnackbar('WebDAV sync started successfully', 'success');
        setPollingSyncStatus(true);
        fetchSyncStatus(); // Get initial status
      } else if (response.data.error === 'sync_already_running') {
        onShowSnackbar('A WebDAV sync is already in progress', 'warning');
      } else {
        onShowSnackbar(response.data.message || 'Failed to start sync', 'error');
      }
    } catch (error: any) {
      console.error('Failed to start sync:', error);
      onShowSnackbar('Failed to start WebDAV sync', 'error');
    } finally {
      setStartingSync(false);
    }
  };

  const cancelManualSync = async () => {
    setCancellingSync(true);
    try {
      const response = await api.post('/webdav/cancel-sync');
      if (response.data.success) {
        onShowSnackbar('WebDAV sync cancelled successfully', 'info');
        fetchSyncStatus(); // Update status
      } else {
        onShowSnackbar(response.data.message || 'Failed to cancel sync', 'error');
      }
    } catch (error: any) {
      console.error('Failed to cancel sync:', error);
      onShowSnackbar('Failed to cancel WebDAV sync', 'error');
    } finally {
      setCancellingSync(false);
    }
  };

  // Poll sync status when enabled
  useEffect(() => {
    if (!settings.webdavEnabled) {
      setSyncStatus(null);
      setPollingSyncStatus(false);
      return;
    }

    // Initial fetch
    fetchSyncStatus();

    // Set up polling interval
    const interval = setInterval(() => {
      fetchSyncStatus();
    }, 3000); // Poll every 3 seconds

    return () => clearInterval(interval);
  }, [settings.webdavEnabled]);

  // Stop polling when sync is not running
  useEffect(() => {
    if (syncStatus && !syncStatus.is_running && pollingSyncStatus) {
      setPollingSyncStatus(false);
    }
  }, [syncStatus, pollingSyncStatus]);

  // Auto-restart sync when folder list changes (if sync was running)
  const [previousFolders, setPreviousFolders] = useState<string[]>([]);
  useEffect(() => {
    if (previousFolders.length > 0 && 
        JSON.stringify(previousFolders.sort()) !== JSON.stringify([...settings.webdavWatchFolders].sort()) &&
        syncStatus?.is_running) {
      
      onShowSnackbar('Folder list changed - restarting WebDAV sync', 'info');
      
      // Cancel current sync and start a new one
      const restartSync = async () => {
        try {
          await api.post('/webdav/cancel-sync');
          // Small delay to ensure cancellation is processed
          setTimeout(() => {
            startManualSync();
          }, 1000);
        } catch (error) {
          console.error('Failed to restart sync after folder change:', error);
        }
      };
      
      restartSync();
    }
    
    setPreviousFolders([...settings.webdavWatchFolders]);
  }, [settings.webdavWatchFolders, syncStatus?.is_running]);

  return (
    <Box>
      <Typography variant="h6" sx={{ mb: 3 }}>
        WebDAV Integration
      </Typography>
      <Typography variant="body2" sx={{ mb: 3, color: 'text.secondary' }}>
        Connect to your WebDAV server (Nextcloud, ownCloud, etc.) to automatically discover and OCR files.
      </Typography>

      {/* Connection Configuration */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="subtitle1" sx={{ mb: 2 }}>
            <CloudSyncIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
            Connection Settings
          </Typography>
          <Divider sx={{ mb: 2 }} />
          
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <FormControl sx={{ mb: 2 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={settings.webdavEnabled}
                      onChange={(e) => onSettingsChange('webdavEnabled', e.target.checked)}
                      disabled={loading}
                    />
                  }
                  label="Enable WebDAV Integration"
                />
                <Typography variant="body2" color="text.secondary">
                  Enable automatic file discovery and synchronization from WebDAV server
                </Typography>
              </FormControl>
            </Grid>

            {settings.webdavEnabled && (
              <>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label="Server URL"
                    value={localWebdavServerUrl}
                    onChange={handleServerUrlChange}
                    disabled={loading}
                    placeholder="https://cloud.example.com"
                    helperText="Full URL to your WebDAV server"
                  />
                </Grid>
                <Grid item xs={12} md={6}>
                  <FormControl fullWidth>
                    <InputLabel>Server Type</InputLabel>
                    <Select
                      value="nextcloud"
                      label="Server Type"
                      disabled={loading}
                    >
                      {serverTypes.map((type) => (
                        <MenuItem key={type.value} value={type.value}>
                          {type.label}
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label="Username"
                    value={localWebdavUsername}
                    onChange={handleUsernameChange}
                    disabled={loading}
                  />
                </Grid>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label="Password / App Password"
                    type="password"
                    value={localWebdavPassword}
                    onChange={handlePasswordChange}
                    disabled={loading}
                    helperText="For Nextcloud/ownCloud, use an app password"
                  />
                </Grid>
                <Grid item xs={12}>
                  <Button
                    variant="outlined"
                    onClick={testConnection}
                    disabled={testingConnection || loading}
                    sx={{ mr: 2 }}
                  >
                    {testingConnection ? 'Testing...' : 'Test Connection'}
                  </Button>
                  {connectionResult && (
                    <Alert severity={connectionResult.success ? 'success' : 'error'} sx={{ mt: 2 }}>
                      {connectionResult.message}
                      {connectionResult.server_version && (
                        <Typography variant="body2">
                          Server: {connectionResult.server_type} v{connectionResult.server_version}
                        </Typography>
                      )}
                    </Alert>
                  )}
                </Grid>
              </>
            )}
          </Grid>
        </CardContent>
      </Card>

      {/* Folder Configuration */}
      {settings.webdavEnabled && (
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="subtitle1" sx={{ mb: 2 }}>
              <FolderIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
              Folders to Monitor
            </Typography>
            <Divider sx={{ mb: 2 }} />
            
            <Typography variant="body2" sx={{ mb: 2, color: 'text.secondary' }}>
              Specify which folders to scan for files. Use absolute paths starting with "/".
            </Typography>

            <Box sx={{ mb: 2 }}>
              <TextField
                label="Add Folder Path"
                value={newFolder}
                onChange={(e) => setNewFolder(e.target.value)}
                placeholder="/Documents"
                disabled={loading}
                sx={{ mr: 1, minWidth: 200 }}
              />
              <Button variant="outlined" onClick={addFolder} disabled={!newFolder || loading}>
                Add Folder
              </Button>
            </Box>

            <Box sx={{ mb: 2 }}>
              {settings.webdavWatchFolders.map((folder, index) => (
                <Chip
                  key={index}
                  label={folder}
                  onDelete={() => removeFolder(folder)}
                  disabled={loading}
                  sx={{ mr: 1, mb: 1 }}
                />
              ))}
            </Box>

            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <TextField
                  fullWidth
                  type="number"
                  label="Sync Interval (minutes)"
                  value={localSyncInterval}
                  onChange={handleSyncIntervalChange}
                  disabled={loading}
                  inputProps={{ min: 15, max: 1440 }}
                  helperText="How often to check for new files"
                />
              </Grid>
              <Grid item xs={12} md={6}>
                <FormControl sx={{ mt: 2 }}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={settings.webdavAutoSync}
                        onChange={(e) => onSettingsChange('webdavAutoSync', e.target.checked)}
                        disabled={loading}
                      />
                    }
                    label="Enable Automatic Sync"
                  />
                  <Typography variant="body2" color="text.secondary">
                    Automatically sync files on the configured interval
                  </Typography>
                </FormControl>
              </Grid>
            </Grid>
          </CardContent>
        </Card>
      )}

      {/* Crawl Estimation */}
      {settings.webdavEnabled && settings.webdavServerUrl && settings.webdavUsername && settings.webdavWatchFolders.length > 0 && (
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="subtitle1" sx={{ mb: 2 }}>
              <AssessmentIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
              Crawl Estimation
            </Typography>
            <Divider sx={{ mb: 2 }} />
            
            <Typography variant="body2" sx={{ mb: 2, color: 'text.secondary' }}>
              Estimate how many files will be processed and how long it will take.
            </Typography>

            <Button
              variant="outlined"
              onClick={estimateCrawl}
              disabled={estimatingCrawl || loading}
              sx={{ mb: 2 }}
            >
              {estimatingCrawl ? 'Estimating...' : 'Estimate Crawl'}
            </Button>

            {estimatingCrawl && (
              <Box sx={{ mb: 2 }}>
                <LinearProgress />
                <Typography variant="body2" sx={{ mt: 1 }}>
                  Analyzing folders and counting files...
                </Typography>
              </Box>
            )}

            {crawlEstimate && (
              <Box>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Estimation Results
                </Typography>
                <Grid container spacing={2} sx={{ mb: 2 }}>
                  <Grid item xs={12} md={3}>
                    <Paper sx={{ p: 2, textAlign: 'center' }}>
                      <Typography variant="h4" color="primary">
                        {crawlEstimate.total_files.toLocaleString()}
                      </Typography>
                      <Typography variant="body2">Total Files</Typography>
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={3}>
                    <Paper sx={{ p: 2, textAlign: 'center' }}>
                      <Typography variant="h4" color="success.main">
                        {crawlEstimate.total_supported_files.toLocaleString()}
                      </Typography>
                      <Typography variant="body2">Supported Files</Typography>
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={3}>
                    <Paper sx={{ p: 2, textAlign: 'center' }}>
                      <Typography variant="h4" color="warning.main">
                        {crawlEstimate.total_estimated_time_hours.toFixed(1)}h
                      </Typography>
                      <Typography variant="body2">Estimated Time</Typography>
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={3}>
                    <Paper sx={{ p: 2, textAlign: 'center' }}>
                      <Typography variant="h4" color="info.main">
                        {(crawlEstimate.total_size_mb / 1024).toFixed(1)}GB
                      </Typography>
                      <Typography variant="body2">Total Size</Typography>
                    </Paper>
                  </Grid>
                </Grid>

                <TableContainer component={Paper}>
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
                      {crawlEstimate.folders.map((folder) => (
                        <TableRow key={folder.path}>
                          <TableCell>{folder.path}</TableCell>
                          <TableCell align="right">{folder.total_files.toLocaleString()}</TableCell>
                          <TableCell align="right">{folder.supported_files.toLocaleString()}</TableCell>
                          <TableCell align="right">{folder.estimated_time_hours.toFixed(1)}h</TableCell>
                          <TableCell align="right">{folder.total_size_mb.toFixed(1)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </TableContainer>
              </Box>
            )}
          </CardContent>
        </Card>
      )}

      {/* Manual Sync & Status */}
      {settings.webdavEnabled && settings.webdavServerUrl && settings.webdavUsername && (
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="subtitle1" sx={{ mb: 2 }}>
              <PlayArrowIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
              Manual Sync & Status
            </Typography>
            <Divider sx={{ mb: 2 }} />
            
            <Grid container spacing={3}>
              {/* Sync Controls */}
              <Grid item xs={12} md={6}>
                <Box>
                  <Typography variant="body2" sx={{ mb: 2, color: 'text.secondary' }}>
                    Start a manual WebDAV sync to immediately pull new or changed files from your configured folders.
                  </Typography>
                  
                  <Button
                    variant="contained"
                    startIcon={startingSync ? <CircularProgress size={16} /> : <PlayArrowIcon />}
                    onClick={startManualSync}
                    disabled={startingSync || loading || syncStatus?.is_running}
                    sx={{ mr: 2 }}
                  >
                    {startingSync ? 'Starting...' : syncStatus?.is_running ? 'Sync Running...' : 'Start Sync Now'}
                  </Button>
                  
                  {syncStatus?.is_running && (
                    <Button
                      variant="outlined"
                      color="error"
                      startIcon={cancellingSync ? <CircularProgress size={16} /> : undefined}
                      onClick={cancelManualSync}
                      disabled={cancellingSync || loading}
                      sx={{ mr: 2 }}
                    >
                      {cancellingSync ? 'Cancelling...' : 'Cancel Sync'}
                    </Button>
                  )}
                  
                  {syncStatus?.is_running && (
                    <Chip
                      label="Sync Active"
                      color="primary"
                      variant="outlined"
                      icon={<CircularProgress size={12} />}
                      sx={{ ml: 1 }}
                    />
                  )}
                </Box>
              </Grid>

              {/* Sync Status */}
              <Grid item xs={12} md={6}>
                {syncStatus && (
                  <Box>
                    <Typography variant="subtitle2" sx={{ mb: 1 }}>
                      Sync Status
                    </Typography>
                    
                    <Grid container spacing={1}>
                      <Grid item xs={6}>
                        <Paper sx={{ p: 1.5, textAlign: 'center' }}>
                          <Typography variant="h6" color="primary">
                            {syncStatus.files_processed || 0}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Files Processed
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6}>
                        <Paper sx={{ p: 1.5, textAlign: 'center' }}>
                          <Typography variant="h6" color="secondary">
                            {syncStatus.files_remaining || 0}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Files Remaining
                          </Typography>
                        </Paper>
                      </Grid>
                    </Grid>

                    {syncStatus.current_folder && (
                      <Alert severity="info" sx={{ mt: 2 }}>
                        <Typography variant="body2">
                          <strong>Currently syncing:</strong> {syncStatus.current_folder}
                        </Typography>
                      </Alert>
                    )}

                    {syncStatus.last_sync && (
                      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
                        Last sync: {new Date(syncStatus.last_sync).toLocaleString()}
                      </Typography>
                    )}

                    {syncStatus.errors && syncStatus.errors.length > 0 && (
                      <Alert severity="error" sx={{ mt: 2 }}>
                        <Typography variant="body2" sx={{ mb: 1 }}>
                          <strong>Recent Errors:</strong>
                        </Typography>
                        {syncStatus.errors.slice(0, 3).map((error: string, index: number) => (
                          <Typography key={index} variant="caption" sx={{ display: 'block' }}>
                            • {error}
                          </Typography>
                        ))}
                        {syncStatus.errors.length > 3 && (
                          <Typography variant="caption" color="text.secondary">
                            ... and {syncStatus.errors.length - 3} more errors
                          </Typography>
                        )}
                      </Alert>
                    )}
                  </Box>
                )}
              </Grid>
            </Grid>
          </CardContent>
        </Card>
      )}
    </Box>
  );
};

const SettingsPage: React.FC = () => {
  const { user: currentUser } = useAuth();
  const [tabValue, setTabValue] = useState<number>(0);
  const [settings, setSettings] = useState<Settings>({
    ocrLanguage: 'eng',
    concurrentOcrJobs: 4,
    ocrTimeoutSeconds: 300,
    maxFileSizeMb: 50,
    allowedFileTypes: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
    autoRotateImages: true,
    enableImagePreprocessing: true,
    searchResultsPerPage: 25,
    searchSnippetLength: 200,
    fuzzySearchThreshold: 0.8,
    retentionDays: null,
    enableAutoCleanup: false,
    enableCompression: false,
    memoryLimitMb: 512,
    cpuPriority: 'normal',
    enableBackgroundOcr: true,
    webdavEnabled: false,
    webdavServerUrl: '',
    webdavUsername: '',
    webdavPassword: '',
    webdavWatchFolders: ['/Documents'],
    webdavFileExtensions: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
    webdavAutoSync: false,
    webdavSyncIntervalMinutes: 60,
  });
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [snackbar, setSnackbar] = useState<SnackbarState>({ 
    open: false, 
    message: '', 
    severity: 'success' 
  });
  const [userDialog, setUserDialog] = useState<UserDialogState>({ 
    open: false, 
    mode: 'create', 
    user: null 
  });
  const [userForm, setUserForm] = useState<UserFormData>({ 
    username: '', 
    email: '', 
    password: '' 
  });

  const ocrLanguages: OcrLanguage[] = [
    { code: 'eng', name: 'English' },
    { code: 'spa', name: 'Spanish' },
    { code: 'fra', name: 'French' },
    { code: 'deu', name: 'German' },
    { code: 'ita', name: 'Italian' },
    { code: 'por', name: 'Portuguese' },
    { code: 'rus', name: 'Russian' },
    { code: 'jpn', name: 'Japanese' },
    { code: 'chi_sim', name: 'Chinese (Simplified)' },
    { code: 'chi_tra', name: 'Chinese (Traditional)' },
    { code: 'kor', name: 'Korean' },
    { code: 'ara', name: 'Arabic' },
    { code: 'hin', name: 'Hindi' },
    { code: 'nld', name: 'Dutch' },
    { code: 'pol', name: 'Polish' },
  ];

  useEffect(() => {
    fetchSettings();
    fetchUsers();
  }, []);

  const fetchSettings = async (): Promise<void> => {
    try {
      const response = await api.get('/settings');
      setSettings({
        ocrLanguage: response.data.ocr_language || 'eng',
        concurrentOcrJobs: response.data.concurrent_ocr_jobs || 4,
        ocrTimeoutSeconds: response.data.ocr_timeout_seconds || 300,
        maxFileSizeMb: response.data.max_file_size_mb || 50,
        allowedFileTypes: response.data.allowed_file_types || ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
        autoRotateImages: response.data.auto_rotate_images !== undefined ? response.data.auto_rotate_images : true,
        enableImagePreprocessing: response.data.enable_image_preprocessing !== undefined ? response.data.enable_image_preprocessing : true,
        searchResultsPerPage: response.data.search_results_per_page || 25,
        searchSnippetLength: response.data.search_snippet_length || 200,
        fuzzySearchThreshold: response.data.fuzzy_search_threshold || 0.8,
        retentionDays: response.data.retention_days,
        enableAutoCleanup: response.data.enable_auto_cleanup || false,
        enableCompression: response.data.enable_compression || false,
        memoryLimitMb: response.data.memory_limit_mb || 512,
        cpuPriority: response.data.cpu_priority || 'normal',
        enableBackgroundOcr: response.data.enable_background_ocr !== undefined ? response.data.enable_background_ocr : true,
        webdavEnabled: response.data.webdav_enabled || false,
        webdavServerUrl: response.data.webdav_server_url || '',
        webdavUsername: response.data.webdav_username || '',
        webdavPassword: response.data.webdav_password || '',
        webdavWatchFolders: response.data.webdav_watch_folders || ['/Documents'],
        webdavFileExtensions: response.data.webdav_file_extensions || ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
        webdavAutoSync: response.data.webdav_auto_sync || false,
        webdavSyncIntervalMinutes: response.data.webdav_sync_interval_minutes || 60,
      });
    } catch (error: any) {
      console.error('Error fetching settings:', error);
      if (error.response?.status !== 404) {
        showSnackbar('Failed to load settings', 'error');
      }
    }
  };

  const fetchUsers = async (): Promise<void> => {
    try {
      const response = await api.get<User[]>('/users');
      setUsers(response.data);
    } catch (error: any) {
      console.error('Error fetching users:', error);
      if (error.response?.status !== 404) {
        showSnackbar('Failed to load users', 'error');
      }
    }
  };

  const handleSettingsChange = async (key: keyof Settings, value: any): Promise<void> => {
    try {
      // Convert camelCase to snake_case for API
      const snakeCase = (str: string): string => str.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
      const apiKey = snakeCase(key);
      
      // Build the update payload with only the changed field
      const updatePayload = { [apiKey]: value };
      
      await api.put('/settings', updatePayload);
      
      // Only update state after successful API call
      setSettings(prevSettings => ({ ...prevSettings, [key]: value }));
      
      // Only show success message for non-text inputs to reduce noise
      if (typeof value !== 'string') {
        showSnackbar('Settings updated successfully', 'success');
      }
    } catch (error) {
      console.error('Error updating settings:', error);
      showSnackbar('Failed to update settings', 'error');
    }
  };

  const handleUserSubmit = async (): Promise<void> => {
    setLoading(true);
    try {
      if (userDialog.mode === 'create') {
        await api.post('/users', userForm);
        showSnackbar('User created successfully', 'success');
      } else {
        const { password, ...updateData } = userForm;
        const payload: any = updateData;
        if (password) {
          payload.password = password;
        }
        await api.put(`/users/${userDialog.user?.id}`, payload);
        showSnackbar('User updated successfully', 'success');
      }
      fetchUsers();
      handleCloseUserDialog();
    } catch (error: any) {
      console.error('Error saving user:', error);
      showSnackbar(error.response?.data?.message || 'Failed to save user', 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteUser = async (userId: string): Promise<void> => {
    if (userId === currentUser?.id) {
      showSnackbar('You cannot delete your own account', 'error');
      return;
    }

    if (window.confirm('Are you sure you want to delete this user?')) {
      setLoading(true);
      try {
        await api.delete(`/users/${userId}`);
        showSnackbar('User deleted successfully', 'success');
        fetchUsers();
      } catch (error) {
        console.error('Error deleting user:', error);
        showSnackbar('Failed to delete user', 'error');
      } finally {
        setLoading(false);
      }
    }
  };

  const handleOpenUserDialog = (mode: 'create' | 'edit', user: User | null = null): void => {
    setUserDialog({ open: true, mode, user });
    if (mode === 'edit' && user) {
      setUserForm({ username: user.username, email: user.email, password: '' });
    } else {
      setUserForm({ username: '', email: '', password: '' });
    }
  };

  const handleCloseUserDialog = (): void => {
    setUserDialog({ open: false, mode: 'create', user: null });
    setUserForm({ username: '', email: '', password: '' });
  };

  const showSnackbar = (message: string, severity: SnackbarState['severity']): void => {
    setSnackbar({ open: true, message, severity });
  };

  const handleTabChange = (event: React.SyntheticEvent, newValue: number): void => {
    setTabValue(newValue);
  };

  const handleOcrLanguageChange = (event: SelectChangeEvent<string>): void => {
    handleSettingsChange('ocrLanguage', event.target.value);
  };

  const handleCpuPriorityChange = (event: SelectChangeEvent<string>): void => {
    handleSettingsChange('cpuPriority', event.target.value);
  };

  const handleResultsPerPageChange = (event: SelectChangeEvent<number>): void => {
    handleSettingsChange('searchResultsPerPage', event.target.value);
  };

  return (
    <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
      <Typography variant="h4" sx={{ mb: 4 }}>
        Settings
      </Typography>

      <Paper sx={{ width: '100%' }}>
        <Tabs value={tabValue} onChange={handleTabChange} aria-label="settings tabs">
          <Tab label="General" />
          <Tab label="WebDAV Integration" />
          <Tab label="User Management" />
        </Tabs>

        <Box sx={{ p: 3 }}>
          {tabValue === 0 && (
            <Box>
              <Typography variant="h6" sx={{ mb: 3 }}>
                General Settings
              </Typography>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    OCR Configuration
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>OCR Language</InputLabel>
                        <Select
                          value={settings.ocrLanguage}
                          label="OCR Language"
                          onChange={handleOcrLanguageChange}
                          disabled={loading}
                        >
                          {ocrLanguages.map((lang) => (
                            <MenuItem key={lang.code} value={lang.code}>
                              {lang.name}
                            </MenuItem>
                          ))}
                        </Select>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Concurrent OCR Jobs"
                        value={settings.concurrentOcrJobs}
                        onChange={(e) => handleSettingsChange('concurrentOcrJobs', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 1, max: 16 }}
                        helperText="Number of OCR jobs that can run simultaneously"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="OCR Timeout (seconds)"
                        value={settings.ocrTimeoutSeconds}
                        onChange={(e) => handleSettingsChange('ocrTimeoutSeconds', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 30, max: 3600 }}
                        helperText="Maximum time for OCR processing per file"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>CPU Priority</InputLabel>
                        <Select
                          value={settings.cpuPriority}
                          label="CPU Priority"
                          onChange={handleCpuPriorityChange}
                          disabled={loading}
                        >
                          <MenuItem value="low">Low</MenuItem>
                          <MenuItem value="normal">Normal</MenuItem>
                          <MenuItem value="high">High</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    File Processing
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Max File Size (MB)"
                        value={settings.maxFileSizeMb}
                        onChange={(e) => handleSettingsChange('maxFileSizeMb', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 1, max: 500 }}
                        helperText="Maximum allowed file size for uploads"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Memory Limit (MB)"
                        value={settings.memoryLimitMb}
                        onChange={(e) => handleSettingsChange('memoryLimitMb', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 128, max: 4096 }}
                        helperText="Memory limit per OCR job"
                      />
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.autoRotateImages}
                              onChange={(e) => handleSettingsChange('autoRotateImages', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Auto-rotate Images"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Automatically detect and correct image orientation
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableImagePreprocessing}
                              onChange={(e) => handleSettingsChange('enableImagePreprocessing', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Image Preprocessing"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Enhance images for better OCR accuracy (deskew, denoise, contrast)
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableBackgroundOcr}
                              onChange={(e) => handleSettingsChange('enableBackgroundOcr', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Background OCR"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Process OCR in the background after file upload
                        </Typography>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Search Configuration
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>Results Per Page</InputLabel>
                        <Select
                          value={settings.searchResultsPerPage}
                          label="Results Per Page"
                          onChange={handleResultsPerPageChange}
                          disabled={loading}
                        >
                          <MenuItem value={10}>10</MenuItem>
                          <MenuItem value={25}>25</MenuItem>
                          <MenuItem value={50}>50</MenuItem>
                          <MenuItem value={100}>100</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Snippet Length"
                        value={settings.searchSnippetLength}
                        onChange={(e) => handleSettingsChange('searchSnippetLength', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 50, max: 500 }}
                        helperText="Characters to show in search result previews"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Fuzzy Search Threshold"
                        value={settings.fuzzySearchThreshold}
                        onChange={(e) => handleSettingsChange('fuzzySearchThreshold', parseFloat(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 0, max: 1, step: 0.1 }}
                        helperText="Tolerance for spelling mistakes (0.0-1.0)"
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Storage Management
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Retention Days"
                        value={settings.retentionDays || ''}
                        onChange={(e) => handleSettingsChange('retentionDays', e.target.value ? parseInt(e.target.value) : null)}
                        disabled={loading}
                        inputProps={{ min: 1 }}
                        helperText="Auto-delete documents after X days (leave empty to disable)"
                      />
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableAutoCleanup}
                              onChange={(e) => handleSettingsChange('enableAutoCleanup', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Auto Cleanup"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Automatically remove orphaned files and clean up storage
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableCompression}
                              onChange={(e) => handleSettingsChange('enableCompression', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Compression"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Compress stored documents to save disk space
                        </Typography>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>
            </Box>
          )}

          {tabValue === 1 && (
            <WebDAVTabContent 
              settings={settings}
              loading={loading}
              onSettingsChange={handleSettingsChange}
              onShowSnackbar={showSnackbar}
            />
          )}

          {tabValue === 2 && (
            <Box>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
                <Typography variant="h6">
                  User Management
                </Typography>
                <Button
                  variant="contained"
                  startIcon={<AddIcon />}
                  onClick={() => handleOpenUserDialog('create')}
                  disabled={loading}
                >
                  Add User
                </Button>
              </Box>

              <TableContainer component={Paper}>
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableCell>Username</TableCell>
                      <TableCell>Email</TableCell>
                      <TableCell>Created At</TableCell>
                      <TableCell align="right">Actions</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {users.map((user) => (
                      <TableRow key={user.id}>
                        <TableCell>{user.username}</TableCell>
                        <TableCell>{user.email}</TableCell>
                        <TableCell>{new Date(user.created_at).toLocaleDateString()}</TableCell>
                        <TableCell align="right">
                          <IconButton
                            onClick={() => handleOpenUserDialog('edit', user)}
                            disabled={loading}
                          >
                            <EditIcon />
                          </IconButton>
                          <IconButton
                            onClick={() => handleDeleteUser(user.id)}
                            disabled={loading || user.id === currentUser?.id}
                          >
                            <DeleteIcon />
                          </IconButton>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </Box>
          )}
        </Box>
      </Paper>

      <Dialog open={userDialog.open} onClose={handleCloseUserDialog} maxWidth="sm" fullWidth>
        <DialogTitle>
          {userDialog.mode === 'create' ? 'Create New User' : 'Edit User'}
        </DialogTitle>
        <DialogContent>
          <Grid container spacing={2} sx={{ mt: 1 }}>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Username"
                value={userForm.username}
                onChange={(e) => setUserForm({ ...userForm, username: e.target.value })}
                required
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Email"
                type="email"
                value={userForm.email}
                onChange={(e) => setUserForm({ ...userForm, email: e.target.value })}
                required
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label={userDialog.mode === 'create' ? 'Password' : 'New Password (leave empty to keep current)'}
                type="password"
                value={userForm.password}
                onChange={(e) => setUserForm({ ...userForm, password: e.target.value })}
                required={userDialog.mode === 'create'}
              />
            </Grid>
          </Grid>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseUserDialog} disabled={loading}>
            Cancel
          </Button>
          <Button onClick={handleUserSubmit} variant="contained" disabled={loading}>
            {userDialog.mode === 'create' ? 'Create' : 'Update'}
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
      >
        <Alert
          onClose={() => setSnackbar({ ...snackbar, open: false })}
          severity={snackbar.severity}
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Container>
  );
};

export default SettingsPage;