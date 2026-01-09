import React, { useState, useEffect } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Card,
  CardContent,
  Chip,
  LinearProgress,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Alert,
  Button,
  IconButton,
  CircularProgress,
  Divider,
  Skeleton,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  Refresh as RefreshIcon,
  Folder as FolderIcon,
  CheckCircleOutline as CheckCircleIcon,
  Error as ErrorIcon,
  Schedule as ScheduleIcon,
  Visibility as VisibilityIcon,
  CloudUpload as CloudUploadIcon,
  Description as DescriptionIcon,
  PersonOutline as PersonIcon,
  CreateNewFolder as CreateFolderIcon,
  AdminPanelSettings as AdminIcon,
} from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { queueService, QueueStats, userWatchService, UserWatchDirectoryResponse } from '../services/api';
import { useAuth } from '../contexts/AuthContext';
import { useFeatureFlags } from '../contexts/FeatureFlagsContext';
import { useTranslation } from 'react-i18next';

interface WatchConfig {
  watchFolder: string;
  watchInterval: number;
  maxFileAge: number;
  allowedTypes: string[];
  isActive: boolean;
  strategy: string;
}

const WatchFolderPage: React.FC = () => {
  const { t } = useTranslation();
  const theme = useTheme();
  const { user } = useAuth();
  const { flags } = useFeatureFlags();
  const perUserWatchEnabled = flags.enablePerUserWatch;

  // Queue statistics state
  const [queueStats, setQueueStats] = useState<QueueStats | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [requeuingFailed, setRequeuingFailed] = useState<boolean>(false);
  
  // User watch directory state
  const [userWatchInfo, setUserWatchInfo] = useState<UserWatchDirectoryResponse | null>(null);
  const [userWatchLoading, setUserWatchLoading] = useState<boolean>(false);
  const [userWatchError, setUserWatchError] = useState<string | null>(null);
  const [creatingDirectory, setCreatingDirectory] = useState<boolean>(false);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Mock configuration data (would typically come from API)
  const watchConfig: WatchConfig = {
    watchFolder: import.meta.env.VITE_WATCH_FOLDER || './watch',
    watchInterval: 30,
    maxFileAge: 24,
    allowedTypes: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt', 'doc', 'docx'],
    isActive: true,
    strategy: 'hybrid'
  };

  useEffect(() => {
    fetchQueueStats();
    if (user && perUserWatchEnabled) {
      fetchUserWatchDirectory();
    }
    const interval = setInterval(fetchQueueStats, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, [user, perUserWatchEnabled]);

  const fetchUserWatchDirectory = async (): Promise<void> => {
    if (!user) return;
    
    try {
      setUserWatchLoading(true);
      setUserWatchError(null);
      const response = await userWatchService.getUserWatchDirectory(user.id);
      setUserWatchInfo(response.data);
    } catch (err) {
      console.error('Error fetching user watch directory:', err);
      setUserWatchError('Failed to fetch user watch directory information');
    } finally {
      setUserWatchLoading(false);
    }
  };

  const fetchQueueStats = async (): Promise<void> => {
    try {
      setLoading(true);
      const response = await queueService.getStats();
      setQueueStats(response.data);
      setLastRefresh(new Date());
      setError(null);
    } catch (err) {
      console.error('Error fetching queue stats:', err);
      setError('Failed to fetch queue statistics');
    } finally {
      setLoading(false);
    }
  };

  const createUserWatchDirectory = async (): Promise<void> => {
    if (!user) return;
    
    try {
      setCreatingDirectory(true);
      setUserWatchError(null);
      setSuccessMessage(null);
      
      const response = await userWatchService.createUserWatchDirectory(user.id);
      
      if (response.data.success) {
        setSuccessMessage(response.data.message);
        // Refresh user watch directory info
        await fetchUserWatchDirectory();
      } else {
        setUserWatchError(response.data.message || 'Failed to create watch directory');
      }
    } catch (err) {
      console.error('Error creating user watch directory:', err);
      setUserWatchError('Failed to create user watch directory');
    } finally {
      setCreatingDirectory(false);
      // Clear success message after 5 seconds
      setTimeout(() => setSuccessMessage(null), 5000);
    }
  };

  const requeueFailedJobs = async (): Promise<void> => {
    try {
      setRequeuingFailed(true);
      const response = await queueService.requeueFailed();
      const requeued = response.data.requeued_count || 0;
      
      if (requeued > 0) {
        // Show success message
        setError(null);
        // Refresh stats to see updated counts
        await fetchQueueStats();
      }
    } catch (err) {
      console.error('Error requeuing failed jobs:', err);
      setError('Failed to requeue failed jobs');
    } finally {
      setRequeuingFailed(false);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const formatDuration = (minutes: number | null | undefined): string => {
    if (!minutes) return 'N/A';
    if (minutes < 60) return `${Math.round(minutes)}m`;
    const hours = Math.floor(minutes / 60);
    const mins = Math.round(minutes % 60);
    return `${hours}h ${mins}m`;
  };

  const getStatusColor = (status: string): 'success' | 'error' | 'warning' | 'default' => {
    switch (status) {
      case 'active': return 'success';
      case 'error': return 'error';
      case 'pending': return 'warning';
      default: return 'default';
    }
  };

  const getStatusIcon = (status: string): React.ReactElement => {
    switch (status) {
      case 'active': return <CheckCircleIcon />;
      case 'error': return <ErrorIcon />;
      case 'pending': return <ScheduleIcon />;
      default: return <VisibilityIcon />;
    }
  };

  return (
    <Container maxWidth="xl" sx={{ mt: 4, mb: 4 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 4 }}>
        <Typography variant="h4" sx={{
          fontWeight: 600,
          background: theme.palette.mode === 'light'
            ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
            : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
          backgroundClip: 'text',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
        }}>
          {t('watchFolder.title')}
        </Typography>
        <Button
          variant="outlined"
          startIcon={<RefreshIcon />}
          onClick={() => {
            fetchQueueStats();
            if (user && perUserWatchEnabled) {
              fetchUserWatchDirectory();
            }
          }}
          disabled={loading || userWatchLoading}
          sx={{ mr: 2 }}
        >
          {t('watchFolder.refreshAll')}
        </Button>

        {queueStats && queueStats.failed_count > 0 && (
          <Button
            variant="contained"
            color="warning"
            startIcon={requeuingFailed ? <CircularProgress size={16} /> : <RefreshIcon />}
            onClick={requeueFailedJobs}
            disabled={requeuingFailed || loading}
          >
            {requeuingFailed ? t('watchFolder.requeuing') : t('watchFolder.retryFailedJobs', { count: queueStats.failed_count })}
          </Button>
        )}
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {error}
        </Alert>
      )}

      {successMessage && (
        <Alert severity="success" sx={{ mb: 3 }}>
          {successMessage}
        </Alert>
      )}

      {/* User Watch Directory - Only show for authenticated users when feature is enabled */}
      {user && perUserWatchEnabled && (
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="h6" sx={{ mb: 2, display: 'flex', alignItems: 'center', gap: 1 }}>
              <PersonIcon color="primary" />
              {t('watchFolder.personalWatchDirectory')}
              {user.role === 'Admin' && (
                <Chip
                  icon={<AdminIcon />}
                  label={t('watchFolder.admin')}
                  size="small"
                  color="primary"
                  variant="outlined"
                  sx={{ ml: 1 }}
                />
              )}
            </Typography>

            {userWatchError && (
              <Alert severity="error" sx={{ mb: 2 }}>
                {userWatchError}
              </Alert>
            )}

            {userWatchLoading ? (
              <Box sx={{ p: 2 }}>
                <Skeleton variant="text" width="60%" height={24} />
                <Skeleton variant="text" width="40%" height={20} sx={{ mt: 1 }} />
                <Skeleton variant="rectangular" width="120px" height={36} sx={{ mt: 2 }} />
              </Box>
            ) : userWatchInfo ? (
              <Grid container spacing={2}>
                <Grid item xs={12} md={8}>
                  <Box sx={{ mb: 2 }}>
                    <Typography variant="body2" color="text.secondary">
                      {t('watchFolder.yourPersonalWatchDirectory')}
                    </Typography>
                    <Typography variant="body1" sx={{
                      fontFamily: 'monospace',
                      bgcolor: theme.palette.mode === 'light' ? 'grey.100' : 'grey.800',
                      p: 1,
                      borderRadius: 1,
                      color: 'text.primary',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 1,
                    }}>
                      <FolderIcon fontSize="small" />
                      {userWatchInfo.watch_directory_path}
                    </Typography>
                  </Box>
                </Grid>
                <Grid item xs={12} md={4}>
                  <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                    <Box>
                      <Typography variant="body2" color="text.secondary">
                        {t('watchFolder.directoryStatus')}
                      </Typography>
                      <Chip
                        icon={userWatchInfo.exists ? <CheckCircleIcon /> : <ErrorIcon />}
                        label={userWatchInfo.exists ? t('watchFolder.directoryExists') : t('watchFolder.directoryMissing')}
                        color={userWatchInfo.exists ? 'success' : 'error'}
                        variant="filled"
                        size="small"
                      />
                    </Box>
                    <Box>
                      <Typography variant="body2" color="text.secondary">
                        {t('watchFolder.watchStatus')}
                      </Typography>
                      <Chip
                        icon={userWatchInfo.enabled ? <CheckCircleIcon /> : <ScheduleIcon />}
                        label={userWatchInfo.enabled ? t('watchFolder.enabled') : t('watchFolder.disabled')}
                        color={userWatchInfo.enabled ? 'success' : 'warning'}
                        variant="filled"
                        size="small"
                      />
                    </Box>
                  </Box>
                </Grid>

                {!userWatchInfo.exists && (
                  <Grid item xs={12}>
                    <Box sx={{
                      mt: 2,
                      p: 2,
                      bgcolor: theme.palette.mode === 'light' ? 'info.light' : 'info.dark',
                      borderRadius: 2,
                      border: `1px solid ${theme.palette.info.main}`,
                    }}>
                      <Typography variant="body2" sx={{ mb: 2, color: 'info.contrastText' }}>
                        {t('watchFolder.directoryNotExist')}
                      </Typography>
                      <Button
                        variant="contained"
                        color="primary"
                        startIcon={creatingDirectory ? <CircularProgress size={16} /> : <CreateFolderIcon />}
                        onClick={createUserWatchDirectory}
                        disabled={creatingDirectory}
                        sx={{ color: 'primary.contrastText' }}
                      >
                        {creatingDirectory ? t('watchFolder.creatingDirectory') : t('watchFolder.createPersonalDirectory')}
                      </Button>
                    </Box>
                  </Grid>
                )}
              </Grid>
            ) : (
              <Alert severity="info">
                {t('watchFolder.unableToLoad')}
              </Alert>
            )}
          </CardContent>
        </Card>
      )}

      {/* Divider between Personal and Global sections */}
      {user && perUserWatchEnabled && (
        <Box sx={{ my: 4, display: 'flex', alignItems: 'center', gap: 2 }}>
          <Divider sx={{ flex: 1 }} />
          <Typography variant="body2" color="text.secondary" sx={{ px: 2 }}>
            {t('watchFolder.systemConfiguration')}
          </Typography>
          <Divider sx={{ flex: 1 }} />
        </Box>
      )}

      {/* Global Watch Folder Configuration */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2, display: 'flex', alignItems: 'center', gap: 1 }}>
            <FolderIcon color="primary" />
            {t('watchFolder.globalWatchFolderConfiguration')}
            {user?.role === 'Admin' && (
              <Chip
                label={t('watchFolder.adminOnly')}
                size="small"
                color="secondary"
                variant="outlined"
                sx={{ ml: 1 }}
              />
            )}
          </Typography>
          {user?.role !== 'Admin' && (
            <Alert severity="info" sx={{ mb: 2 }}>
              {t('watchFolder.systemWideInfo')}
            </Alert>
          )}
          <Grid container spacing={2}>
            <Grid item xs={12} md={6}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('watchFolder.watchedDirectory')}
                </Typography>
                <Typography variant="body1" sx={{
                  fontFamily: 'monospace',
                  bgcolor: theme.palette.mode === 'light' ? 'grey.100' : 'grey.800',
                  p: 1,
                  borderRadius: 1,
                  color: 'text.primary',
                }}>
                  {watchConfig.watchFolder}
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={6}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('watchFolder.status')}
                </Typography>
                <Chip
                  icon={getStatusIcon(watchConfig.isActive ? 'active' : 'error')}
                  label={watchConfig.isActive ? t('watchFolder.active') : t('watchFolder.inactive')}
                  color={getStatusColor(watchConfig.isActive ? 'active' : 'error')}
                  variant="filled"
                />
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('watchFolder.watchStrategy')}
                </Typography>
                <Typography variant="body1" sx={{ textTransform: 'capitalize' }}>
                  {watchConfig.strategy}
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('watchFolder.scanInterval')}
                </Typography>
                <Typography variant="body1">
                  {t('watchFolder.seconds', { count: watchConfig.watchInterval })}
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('watchFolder.maxFileAge')}
                </Typography>
                <Typography variant="body1">
                  {t('watchFolder.hours', { count: watchConfig.maxFileAge })}
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12}>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                  {t('watchFolder.supportedFileTypes')}
                </Typography>
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                  {watchConfig.allowedTypes.map((type) => (
                    <Chip
                      key={type}
                      label={`.${type}`}
                      size="small"
                      variant="outlined"
                      color="primary"
                    />
                  ))}
                </Box>
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {/* Queue Statistics */}
      {queueStats && (
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="h6" sx={{ mb: 2, display: 'flex', alignItems: 'center', gap: 1 }}>
              <CloudUploadIcon color="primary" />
              {t('watchFolder.processingQueue')}
            </Typography>
            <Grid container spacing={2}>
              <Grid item xs={12} sm={6} md={3}>
                <Box sx={{ 
                  textAlign: 'center', 
                  p: 2, 
                  bgcolor: theme.palette.mode === 'dark' 
                    ? 'rgba(2, 136, 209, 0.15)' 
                    : 'info.light', 
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(2, 136, 209, 0.3)' : 'none'
                }}>
                  <Typography variant="h4" sx={{
                    fontWeight: 600,
                    color: theme.palette.mode === 'dark' ? '#29b6f6' : 'info.dark'
                  }}>
                    {queueStats.pending_count}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('watchFolder.pending')}
                  </Typography>
                </Box>
              </Grid>
              <Grid item xs={12} sm={6} md={3}>
                <Box sx={{ 
                  textAlign: 'center', 
                  p: 2, 
                  bgcolor: theme.palette.mode === 'dark' 
                    ? 'rgba(237, 108, 2, 0.15)' 
                    : 'warning.light', 
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(237, 108, 2, 0.3)' : 'none'
                }}>
                  <Typography variant="h4" sx={{
                    fontWeight: 600,
                    color: theme.palette.mode === 'dark' ? '#ff9800' : 'warning.dark'
                  }}>
                    {queueStats.processing_count}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('watchFolder.processing')}
                  </Typography>
                </Box>
              </Grid>
              <Grid item xs={12} sm={6} md={3}>
                <Box sx={{ 
                  textAlign: 'center', 
                  p: 2, 
                  bgcolor: theme.palette.mode === 'dark' 
                    ? 'rgba(211, 47, 47, 0.15)' 
                    : 'error.light', 
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(211, 47, 47, 0.3)' : 'none'
                }}>
                  <Typography variant="h4" sx={{
                    fontWeight: 600,
                    color: theme.palette.mode === 'dark' ? '#ef5350' : 'error.dark'
                  }}>
                    {queueStats.failed_count}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('watchFolder.failed')}
                  </Typography>
                </Box>
              </Grid>
              <Grid item xs={12} sm={6} md={3}>
                <Box sx={{
                  textAlign: 'center',
                  p: 2,
                  bgcolor: theme.palette.mode === 'dark'
                    ? 'rgba(46, 125, 50, 0.15)'
                    : 'success.light',
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(46, 125, 50, 0.3)' : 'none'
                }}>
                  <Typography variant="h4" sx={{
                    fontWeight: 600,
                    color: theme.palette.mode === 'dark' ? '#81c784' : 'success.dark'
                  }}>
                    {queueStats.completed_today}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('watchFolder.completedToday')}
                  </Typography>
                </Box>
              </Grid>
            </Grid>

            <Grid container spacing={2} sx={{ mt: 2 }}>
              <Grid item xs={12} md={6}>
                <Box sx={{
                  p: 2,
                  bgcolor: theme.palette.mode === 'light' ? 'grey.50' : 'grey.800',
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(255,255,255,0.1)' : 'none',
                }}>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                    {t('watchFolder.averageWaitTime')}
                  </Typography>
                  <Typography variant="h6">
                    {formatDuration(queueStats.avg_wait_time_minutes)}
                  </Typography>
                </Box>
              </Grid>
              <Grid item xs={12} md={6}>
                <Box sx={{
                  p: 2,
                  bgcolor: theme.palette.mode === 'light' ? 'grey.50' : 'grey.800',
                  borderRadius: 2,
                  border: theme.palette.mode === 'dark' ? '1px solid rgba(255,255,255,0.1)' : 'none',
                }}>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                    {t('watchFolder.oldestPendingItem')}
                  </Typography>
                  <Typography variant="h6">
                    {formatDuration(queueStats.oldest_pending_minutes)}
                  </Typography>
                </Box>
              </Grid>
            </Grid>

            {lastRefresh && (
              <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
                {t('watchFolder.lastUpdated', { time: lastRefresh.toLocaleTimeString() })}
              </Typography>
            )}
          </CardContent>
        </Card>
      )}

      {/* Processing Information */}
      <Card>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2, display: 'flex', alignItems: 'center', gap: 1 }}>
            <DescriptionIcon color="primary" />
            {t('watchFolder.howWatchFolderWorks')}
          </Typography>
          <Typography variant="body1" sx={{ mb: 2 }}>
            {t('watchFolder.watchFolderDescription')}
          </Typography>

          <Box sx={{ mb: 3 }}>
            <Typography variant="subtitle2" sx={{ mb: 1, color: 'primary.main' }}>
              {t('watchFolder.processingPipeline')}
            </Typography>
            <Box sx={{ pl: 2 }}>
              <Typography variant="body2" sx={{ mb: 0.5 }}>
                {t('watchFolder.pipelineSteps.fileDetection')}
              </Typography>
              <Typography variant="body2" sx={{ mb: 0.5 }}>
                {t('watchFolder.pipelineSteps.validation')}
              </Typography>
              <Typography variant="body2" sx={{ mb: 0.5 }}>
                {t('watchFolder.pipelineSteps.deduplication')}
              </Typography>
              <Typography variant="body2" sx={{ mb: 0.5 }}>
                {t('watchFolder.pipelineSteps.storage')}
              </Typography>
              <Typography variant="body2" sx={{ mb: 0.5 }}>
                {t('watchFolder.pipelineSteps.ocrQueue')}
              </Typography>
            </Box>
          </Box>

          <Alert severity="info" sx={{ mt: 2 }}>
            <Typography variant="body2">
              {t('watchFolder.hybridStrategyInfo')}
            </Typography>
          </Alert>
        </CardContent>
      </Card>
    </Container>
  );
};

export default WatchFolderPage;