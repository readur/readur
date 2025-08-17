import React, { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Alert,
  Chip,
  IconButton,
  TextField,
  InputAdornment,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Card,
  CardContent,
  Grid,
  LinearProgress,
  Skeleton,
  Stack,
  Fade,
  Collapse,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Search as SearchIcon,
  FilterList as FilterIcon,
  Refresh as RefreshIcon,
  Error as ErrorIcon,
  Warning as WarningIcon,
  Info as InfoIcon,
  CheckCircle as CheckCircleIcon,
  Cloud as CloudIcon,
  Folder as FolderIcon,
  Language as WebDAVIcon,
} from '@mui/icons-material';
import { alpha } from '@mui/material/styles';

import { sourceErrorService, SourceScanFailure, SourceErrorSeverity, SourceErrorType, SourceType } from '../../services/api';
import { useNotification } from '../../contexts/NotificationContext';
import { modernTokens } from '../../theme';
import StatsDashboard from './StatsDashboard';
import FailureDetailsPanel from './FailureDetailsPanel';
import RecommendationsSection from './RecommendationsSection';

// Severity configuration for styling
const severityConfig = {
  critical: {
    color: modernTokens.colors.error[500],
    bgColor: modernTokens.colors.error[50],
    icon: ErrorIcon,
    label: 'Critical',
  },
  high: {
    color: modernTokens.colors.warning[600],
    bgColor: modernTokens.colors.warning[50],
    icon: WarningIcon,
    label: 'High',
  },
  medium: {
    color: modernTokens.colors.warning[500],
    bgColor: modernTokens.colors.warning[50],
    icon: InfoIcon,
    label: 'Medium',
  },
  low: {
    color: modernTokens.colors.info[500],
    bgColor: modernTokens.colors.info[50],
    icon: InfoIcon,
    label: 'Low',
  },
};

// Source type configuration
const sourceTypeConfig: Record<SourceType, { label: string; icon: string; color: string }> = {
  webdav: { label: 'WebDAV', icon: '🌐', color: modernTokens.colors.blue[500] },
  s3: { label: 'S3', icon: '☁️', color: modernTokens.colors.orange[500] },
  local_folder: { label: 'Local', icon: '📁', color: modernTokens.colors.green[500] },
};

// Failure type configuration
const failureTypeConfig: Record<string, { label: string; description: string }> = {
  // WebDAV types
  timeout: { label: 'Timeout', description: 'Request timed out' },
  path_too_long: { label: 'Path Too Long', description: 'File path exceeds system limits' },
  permission_denied: { label: 'Permission Denied', description: 'Access denied' },
  invalid_characters: { label: 'Invalid Characters', description: 'Path contains invalid characters' },
  network_error: { label: 'Network Error', description: 'Network connectivity issue' },
  server_error: { label: 'Server Error', description: 'Server returned an error' },
  xml_parse_error: { label: 'XML Parse Error', description: 'Failed to parse server response' },
  too_many_items: { label: 'Too Many Items', description: 'Directory contains too many files' },
  depth_limit: { label: 'Depth Limit', description: 'Directory nesting too deep' },
  size_limit: { label: 'Size Limit', description: 'Directory or file too large' },
  unknown: { label: 'Unknown', description: 'Unclassified error' },
  // S3 types
  s3_access_denied: { label: 'S3 Access Denied', description: 'S3 access denied' },
  s3_bucket_not_found: { label: 'S3 Bucket Not Found', description: 'S3 bucket does not exist' },
  s3_invalid_credentials: { label: 'S3 Invalid Credentials', description: 'S3 credentials are invalid' },
  s3_network_error: { label: 'S3 Network Error', description: 'S3 network connectivity issue' },
  // Local types
  local_permission_denied: { label: 'Local Permission Denied', description: 'Local file system permission denied' },
  local_path_not_found: { label: 'Local Path Not Found', description: 'Local path does not exist' },
  local_disk_full: { label: 'Local Disk Full', description: 'Local disk is full' },
  local_io_error: { label: 'Local I/O Error', description: 'Local file system I/O error' },
};

interface SourceErrorsProps {
  autoRefresh?: boolean;
  refreshInterval?: number;
  sourceTypeFilter?: SourceType | 'all';
}

const SourceErrors: React.FC<SourceErrorsProps> = ({
  autoRefresh = true,
  refreshInterval = 30000, // 30 seconds
  sourceTypeFilter = 'all',
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [severityFilter, setSeverityFilter] = useState<SourceErrorSeverity | 'all'>('all');
  const [typeFilter, setTypeFilter] = useState<SourceErrorType | 'all'>('all');
  const [currentSourceFilter, setCurrentSourceFilter] = useState<SourceType | 'all'>(sourceTypeFilter);
  const [expandedFailure, setExpandedFailure] = useState<string | null>(null);
  const [showResolved, setShowResolved] = useState(false);
  
  // Data state
  const [sourceFailuresData, setSourceFailuresData] = useState<any>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // Action states
  const [retryingFailures, setRetryingFailures] = useState<Set<string>>(new Set());
  const [excludingFailures, setExcludingFailures] = useState<Set<string>>(new Set());

  const { showNotification } = useNotification();

  // Fetch source failures
  const fetchSourceFailures = useCallback(async () => {
    try {
      setError(null);
      const response = currentSourceFilter === 'all' 
        ? await sourceErrorService.getSourceFailures()
        : await sourceErrorService.getSourceFailuresByType(currentSourceFilter);
      setSourceFailuresData(response.data);
    } catch (err: any) {
      console.error('Failed to fetch source failures:', err);
      setError(err?.response?.data?.message || err.message || 'Failed to load source failures');
    } finally {
      setIsLoading(false);
    }
  }, [currentSourceFilter]);

  // Auto-refresh effect
  useEffect(() => {
    fetchSourceFailures();
    
    if (autoRefresh && refreshInterval > 0) {
      const interval = setInterval(fetchSourceFailures, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchSourceFailures, autoRefresh, refreshInterval]);

  // Manual refetch
  const refetch = useCallback(() => {
    setIsLoading(true);
    fetchSourceFailures();
  }, [fetchSourceFailures]);

  // Update source filter effect
  useEffect(() => {
    setCurrentSourceFilter(sourceTypeFilter);
  }, [sourceTypeFilter]);

  // Filter failures based on search and filters
  const filteredFailures = useMemo(() => {
    if (!sourceFailuresData?.failures) return [];

    return sourceFailuresData.failures.filter((failure) => {
      // Search filter
      if (searchQuery) {
        const searchLower = searchQuery.toLowerCase();
        if (!failure.resource_path.toLowerCase().includes(searchLower) &&
            !failure.error_message?.toLowerCase().includes(searchLower)) {
          return false;
        }
      }

      // Severity filter
      if (severityFilter !== 'all' && failure.error_severity !== severityFilter) {
        return false;
      }

      // Type filter
      if (typeFilter !== 'all' && failure.error_type !== typeFilter) {
        return false;
      }

      // Show resolved filter
      if (!showResolved && failure.resolved) {
        return false;
      }

      return true;
    });
  }, [sourceFailuresData?.failures, searchQuery, severityFilter, typeFilter, showResolved]);

  // Handle accordion expansion
  const handleAccordionChange = (failureId: string) => (
    event: React.SyntheticEvent,
    isExpanded: boolean
  ) => {
    setExpandedFailure(isExpanded ? failureId : null);
  };

  // Handle retry action
  const handleRetry = async (failure: SourceScanFailure, notes?: string) => {
    try {
      setRetryingFailures(prev => new Set(prev).add(failure.id));
      const response = await sourceErrorService.retryFailure(failure.id, { notes });
      
      showNotification({
        type: 'success',
        message: `Retry scheduled for: ${response.data.resource_path}`,
      });
      
      // Refresh the data
      await fetchSourceFailures();
    } catch (error: any) {
      console.error('Failed to retry source failure:', error);
      showNotification({
        type: 'error',
        message: `Failed to schedule retry: ${error?.response?.data?.message || error.message}`,
      });
    } finally {
      setRetryingFailures(prev => {
        const newSet = new Set(prev);
        newSet.delete(failure.id);
        return newSet;
      });
    }
  };

  // Handle exclude action
  const handleExclude = async (failure: SourceScanFailure, notes?: string, permanent = true) => {
    try {
      setExcludingFailures(prev => new Set(prev).add(failure.id));
      const response = await sourceErrorService.excludeFailure(failure.id, { notes, permanent });
      
      showNotification({
        type: 'success',
        message: `Resource excluded: ${response.data.resource_path}`,
      });
      
      // Refresh the data
      await fetchSourceFailures();
    } catch (error: any) {
      console.error('Failed to exclude resource:', error);
      showNotification({
        type: 'error',
        message: `Failed to exclude resource: ${error?.response?.data?.message || error.message}`,
      });
    } finally {
      setExcludingFailures(prev => {
        const newSet = new Set(prev);
        newSet.delete(failure.id);
        return newSet;
      });
    }
  };

  // Render severity chip
  const renderSeverityChip = (severity: SourceErrorSeverity) => {
    const config = severityConfig[severity];
    const Icon = config.icon;

    return (
      <Chip
        icon={<Icon sx={{ fontSize: 16 }} />}
        label={config.label}
        size="small"
        sx={{
          color: config.color,
          backgroundColor: config.bgColor,
          borderColor: config.color,
          fontWeight: 500,
        }}
      />
    );
  };

  // Render failure type chip
  const renderFailureTypeChip = (type: SourceErrorType) => {
    const config = failureTypeConfig[type] || { label: type, description: 'Unknown error type' };

    return (
      <Chip
        label={config.label}
        size="small"
        variant="outlined"
        sx={{
          borderColor: modernTokens.colors.neutral[300],
          color: modernTokens.colors.neutral[700],
        }}
      />
    );
  };

  // Render source type chip
  const renderSourceTypeChip = (sourceType: SourceType) => {
    const config = sourceTypeConfig[sourceType];

    return (
      <Chip
        label={`${config.icon} ${config.label}`}
        size="small"
        sx={{
          backgroundColor: alpha(config.color, 0.1),
          color: config.color,
          fontWeight: 600,
        }}
      />
    );
  };

  if (error) {
    return (
      <Alert 
        severity="error" 
        sx={{ 
          borderRadius: 2,
          boxShadow: modernTokens.shadows.sm,
        }}
        action={
          <IconButton
            color="inherit"
            size="small"
            onClick={refetch}
          >
            <RefreshIcon />
          </IconButton>
        }
      >
        Failed to load source failures: {error}
      </Alert>
    );
  }

  return (
    <Box sx={{ p: 3, maxWidth: 1200, mx: 'auto' }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography 
          variant="h4" 
          sx={{ 
            fontWeight: 700,
            color: modernTokens.colors.neutral[900],
            mb: 1,
          }}
        >
          Source Failures
        </Typography>
        <Typography 
          variant="body1" 
          sx={{ 
            color: modernTokens.colors.neutral[600],
            mb: 3,
          }}
        >
          Monitor and manage resources that failed to scan across all source types
        </Typography>

        {/* Statistics Dashboard */}
        {sourceFailuresData?.stats && (
          <StatsDashboard 
            stats={sourceFailuresData.stats} 
            isLoading={isLoading}
          />
        )}
      </Box>

      {/* Controls */}
      <Paper 
        elevation={0} 
        sx={{ 
          p: 3, 
          mb: 3, 
          backgroundColor: modernTokens.colors.neutral[50],
          border: `1px solid ${modernTokens.colors.neutral[200]}`,
          borderRadius: 2,
        }}
      >
        <Grid container spacing={2} alignItems="center">
          <Grid item xs={12} md={3}>
            <TextField
              fullWidth
              placeholder="Search resources or error messages..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <SearchIcon sx={{ color: modernTokens.colors.neutral[400] }} />
                  </InputAdornment>
                ),
              }}
              sx={{
                '& .MuiOutlinedInput-root': {
                  backgroundColor: modernTokens.colors.neutral[0],
                },
              }}
            />
          </Grid>
          <Grid item xs={12} md={2}>
            <FormControl fullWidth>
              <InputLabel>Source Type</InputLabel>
              <Select
                value={currentSourceFilter}
                label="Source Type"
                onChange={(e) => setCurrentSourceFilter(e.target.value as SourceType | 'all')}
                sx={{
                  backgroundColor: modernTokens.colors.neutral[0],
                }}
              >
                <MenuItem value="all">All Sources</MenuItem>
                <MenuItem value="webdav">WebDAV</MenuItem>
                <MenuItem value="s3">S3</MenuItem>
                <MenuItem value="local_folder">Local Folder</MenuItem>
              </Select>
            </FormControl>
          </Grid>
          <Grid item xs={12} md={2.5}>
            <FormControl fullWidth>
              <InputLabel>Severity</InputLabel>
              <Select
                value={severityFilter}
                label="Severity"
                onChange={(e) => setSeverityFilter(e.target.value as SourceErrorSeverity | 'all')}
                sx={{
                  backgroundColor: modernTokens.colors.neutral[0],
                }}
              >
                <MenuItem value="all">All Severities</MenuItem>
                <MenuItem value="critical">Critical</MenuItem>
                <MenuItem value="high">High</MenuItem>
                <MenuItem value="medium">Medium</MenuItem>
                <MenuItem value="low">Low</MenuItem>
              </Select>
            </FormControl>
          </Grid>
          <Grid item xs={12} md={2.5}>
            <FormControl fullWidth>
              <InputLabel>Type</InputLabel>
              <Select
                value={typeFilter}
                label="Type"
                onChange={(e) => setTypeFilter(e.target.value as SourceErrorType | 'all')}
                sx={{
                  backgroundColor: modernTokens.colors.neutral[0],
                }}
              >
                <MenuItem value="all">All Types</MenuItem>
                {Object.entries(failureTypeConfig).map(([type, config]) => (
                  <MenuItem key={type} value={type}>
                    {config.label}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Grid>
          <Grid item xs={12} md={2}>
            <IconButton
              onClick={() => refetch()}
              disabled={isLoading}
              sx={{
                backgroundColor: modernTokens.colors.primary[50],
                color: modernTokens.colors.primary[600],
                '&:hover': {
                  backgroundColor: modernTokens.colors.primary[100],
                },
              }}
            >
              <RefreshIcon />
            </IconButton>
          </Grid>
        </Grid>
      </Paper>

      {/* Loading State */}
      {isLoading && (
        <Stack spacing={2}>
          {[1, 2, 3].map((i) => (
            <Skeleton 
              key={i} 
              variant="rectangular" 
              height={120} 
              sx={{ borderRadius: 2 }} 
            />
          ))}
        </Stack>
      )}

      {/* Failures List */}
      {!isLoading && (
        <Fade in={!isLoading}>
          <Box>
            {filteredFailures.length === 0 ? (
              <Card 
                sx={{ 
                  textAlign: 'center', 
                  py: 6,
                  backgroundColor: modernTokens.colors.neutral[50],
                  border: `1px solid ${modernTokens.colors.neutral[200]}`,
                }}
              >
                <CardContent>
                  <CheckCircleIcon 
                    sx={{ 
                      fontSize: 64, 
                      color: modernTokens.colors.success[500],
                      mb: 2,
                    }} 
                  />
                  <Typography variant="h6" sx={{ mb: 1 }}>
                    No Source Failures Found
                  </Typography>
                  <Typography 
                    variant="body2" 
                    sx={{ color: modernTokens.colors.neutral[600] }}
                  >
                    {sourceFailuresData?.failures.length === 0 
                      ? 'All sources are scanning successfully!' 
                      : 'Try adjusting your search criteria or filters.'}
                  </Typography>
                </CardContent>
              </Card>
            ) : (
              <Stack spacing={2}>
                {filteredFailures.map((failure) => (
                  <Accordion
                    key={failure.id}
                    expanded={expandedFailure === failure.id}
                    onChange={handleAccordionChange(failure.id)}
                    sx={{
                      boxShadow: modernTokens.shadows.sm,
                      '&:before': { display: 'none' },
                      border: `1px solid ${modernTokens.colors.neutral[200]}`,
                      borderRadius: '12px !important',
                      '&.Mui-expanded': {
                        margin: 0,
                        boxShadow: modernTokens.shadows.md,
                      },
                    }}
                  >
                    <AccordionSummary
                      expandIcon={<ExpandMoreIcon />}
                      sx={{
                        '& .MuiAccordionSummary-content': {
                          alignItems: 'center',
                          gap: 2,
                        },
                      }}
                    >
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
                        {renderSourceTypeChip(failure.source_type)}
                        {renderSeverityChip(failure.error_severity)}
                        {renderFailureTypeChip(failure.error_type)}
                        
                        <Box sx={{ flex: 1 }}>
                          <Typography 
                            variant="subtitle1" 
                            sx={{ 
                              fontWeight: 600,
                              color: modernTokens.colors.neutral[900],
                            }}
                          >
                            {failure.resource_path}
                          </Typography>
                          <Typography 
                            variant="body2" 
                            sx={{ 
                              color: modernTokens.colors.neutral[600],
                              mt: 0.5,
                            }}
                          >
                            {failure.consecutive_failures} consecutive failures • 
                            Last failed: {new Date(failure.last_failure_at).toLocaleString()}
                          </Typography>
                        </Box>

                        {failure.user_excluded && (
                          <Chip
                            label="Excluded"
                            size="small"
                            sx={{
                              backgroundColor: modernTokens.colors.neutral[100],
                              color: modernTokens.colors.neutral[700],
                            }}
                          />
                        )}

                        {failure.resolved && (
                          <Chip
                            label="Resolved"
                            size="small"
                            sx={{
                              backgroundColor: modernTokens.colors.success[100],
                              color: modernTokens.colors.success[700],
                            }}
                          />
                        )}
                      </Box>
                    </AccordionSummary>

                    <AccordionDetails sx={{ pt: 0 }}>
                      <FailureDetailsPanel
                        failure={failure}
                        onRetry={handleRetry}
                        onExclude={handleExclude}
                        isRetrying={retryingFailures.has(failure.id)}
                        isExcluding={excludingFailures.has(failure.id)}
                      />
                    </AccordionDetails>
                  </Accordion>
                ))}
              </Stack>
            )}

            {/* Recommendations Section */}
            {filteredFailures.length > 0 && (
              <Box sx={{ mt: 4 }}>
                <RecommendationsSection failures={filteredFailures} />
              </Box>
            )}
          </Box>
        </Fade>
      )}
    </Box>
  );
};

export default SourceErrors;