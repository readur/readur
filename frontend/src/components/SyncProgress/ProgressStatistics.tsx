import React from 'react';
import { Box, Typography, LinearProgress, Chip, useTheme, alpha } from '@mui/material';
import { Warning as WarningIcon, Error as ErrorIcon, Timer as TimerIcon } from '@mui/icons-material';
import { SyncProgressInfo } from '../../services/api';

interface ProgressStatisticsProps {
  progressInfo: SyncProgressInfo;
  phaseColor: string;
}

export const ProgressStatistics: React.FC<ProgressStatisticsProps> = ({
  progressInfo,
  phaseColor
}) => {
  const theme = useTheme();

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const formatDuration = (seconds: number): string => {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  return (
    <>
      {/* Progress Bar */}
      {progressInfo.files_found > 0 && (
        <Box>
          <Box display="flex" justifyContent="space-between" alignItems="center" mb={1}>
            <Typography variant="body2" color="text.secondary">
              Files Progress
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {progressInfo.files_processed} / {progressInfo.files_found} files ({progressInfo.files_progress_percent.toFixed(1)}%)
            </Typography>
          </Box>
          <LinearProgress 
            variant="determinate" 
            value={progressInfo.files_progress_percent}
            sx={{
              height: 8,
              borderRadius: 4,
              backgroundColor: alpha(phaseColor, 0.2),
              '& .MuiLinearProgress-bar': {
                backgroundColor: phaseColor,
              },
            }}
          />
        </Box>
      )}

      {/* Statistics Grid */}
      <Box display="grid" gridTemplateColumns="repeat(auto-fit, minmax(200px, 1fr))" gap={2}>
        <Box>
          <Typography variant="body2" color="text.secondary">
            Directories
          </Typography>
          <Typography variant="h6">
            {progressInfo.directories_processed} / {progressInfo.directories_found}
          </Typography>
        </Box>
        
        <Box>
          <Typography variant="body2" color="text.secondary">
            Data Processed
          </Typography>
          <Typography variant="h6">
            {formatBytes(progressInfo.bytes_processed)}
          </Typography>
        </Box>

        <Box>
          <Typography variant="body2" color="text.secondary">
            Processing Rate
          </Typography>
          <Typography variant="h6">
            {progressInfo.processing_rate_files_per_sec.toFixed(1)} files/sec
          </Typography>
        </Box>

        <Box>
          <Typography variant="body2" color="text.secondary">
            Elapsed Time
          </Typography>
          <Typography variant="h6">
            {formatDuration(progressInfo.elapsed_time_secs)}
          </Typography>
        </Box>
      </Box>

      {/* Estimated Time Remaining */}
      {progressInfo.estimated_time_remaining_secs && progressInfo.estimated_time_remaining_secs > 0 && (
        <Box display="flex" alignItems="center" gap={1}>
          <TimerIcon color="action" />
          <Typography variant="body2" color="text.secondary">
            Estimated time remaining: {formatDuration(progressInfo.estimated_time_remaining_secs)}
          </Typography>
        </Box>
      )}

      {/* Current Operations */}
      {progressInfo.current_directory && (
        <Box>
          <Typography variant="body2" color="text.secondary" gutterBottom>
            Current Directory
          </Typography>
          <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
            {progressInfo.current_directory}
          </Typography>
          {progressInfo.current_file && (
            <>
              <Typography variant="body2" color="text.secondary" gutterBottom sx={{ mt: 1 }}>
                Current File
              </Typography>
              <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                {progressInfo.current_file}
              </Typography>
            </>
          )}
        </Box>
      )}

      {/* Errors and Warnings */}
      {(progressInfo.errors > 0 || progressInfo.warnings > 0) && (
        <Box display="flex" gap={2}>
          {progressInfo.errors > 0 && (
            <Chip
              icon={<ErrorIcon />}
              label={`${progressInfo.errors} error${progressInfo.errors !== 1 ? 's' : ''}`}
              color="error"
              size="small"
            />
          )}
          {progressInfo.warnings > 0 && (
            <Chip
              icon={<WarningIcon />}
              label={`${progressInfo.warnings} warning${progressInfo.warnings !== 1 ? 's' : ''}`}
              color="warning"
              size="small"
            />
          )}
        </Box>
      )}
    </>
  );
};

export default ProgressStatistics;