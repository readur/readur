import React from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Grid,
  LinearProgress,
  Stack,
  Skeleton,
} from '@mui/material';
import {
  Error as ErrorIcon,
  Warning as WarningIcon,
  Info as InfoIcon,
  CheckCircle as CheckCircleIcon,
  Refresh as RefreshIcon,
  Block as BlockIcon,
} from '@mui/icons-material';

import { SourceScanFailureStats } from '../../services/api';
import { modernTokens } from '../../theme';

interface StatsDashboardProps {
  stats: SourceScanFailureStats;
  isLoading?: boolean;
}

interface StatCardProps {
  title: string;
  value: number;
  icon: React.ElementType;
  color: string;
  bgColor: string;
  description?: string;
  percentage?: number;
  trend?: 'up' | 'down' | 'stable';
}

const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  icon: Icon,
  color,
  bgColor,
  description,
  percentage,
}) => (
  <Card
    sx={{
      height: '100%',
      background: `linear-gradient(135deg, ${bgColor} 0%, ${bgColor}88 100%)`,
      border: `1px solid ${color}20`,
      borderRadius: 3,
      transition: 'all 0.2s ease-in-out',
      '&:hover': {
        transform: 'translateY(-2px)',
        boxShadow: modernTokens.shadows.lg,
      },
    }}
  >
    <CardContent>
      <Stack direction="row" alignItems="center" spacing={2}>
        <Box
          sx={{
            p: 1.5,
            borderRadius: 2,
            backgroundColor: `${color}15`,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <Icon sx={{ color, fontSize: 24 }} />
        </Box>
        
        <Box sx={{ flex: 1 }}>
          <Typography
            variant="h4"
            sx={{
              fontWeight: 700,
              color: modernTokens.colors.neutral[900],
              mb: 0.5,
            }}
          >
            {value.toLocaleString()}
          </Typography>
          <Typography
            variant="body2"
            sx={{
              color: modernTokens.colors.neutral[600],
              fontWeight: 500,
            }}
          >
            {title}
          </Typography>
          {description && (
            <Typography
              variant="caption"
              sx={{
                color: modernTokens.colors.neutral[500],
                display: 'block',
                mt: 0.5,
              }}
            >
              {description}
            </Typography>
          )}
        </Box>
      </Stack>

      {percentage !== undefined && (
        <Box sx={{ mt: 2 }}>
          <LinearProgress
            variant="determinate"
            value={percentage}
            sx={{
              height: 8,
              borderRadius: 4,
              backgroundColor: `${color}10`,
              '& .MuiLinearProgress-bar': {
                borderRadius: 4,
                backgroundColor: color,
              },
            }}
          />
          <Typography
            variant="caption"
            sx={{
              color: modernTokens.colors.neutral[500],
              mt: 0.5,
              display: 'block',
            }}
          >
            {percentage.toFixed(1)}% of total
          </Typography>
        </Box>
      )}
    </CardContent>
  </Card>
);

const StatsDashboard: React.FC<StatsDashboardProps> = ({ stats, isLoading }) => {
  if (isLoading) {
    return (
      <Grid container spacing={3} sx={{ mb: 4 }}>
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <Grid item xs={12} sm={6} md={4} lg={2} key={i}>
            <Card sx={{ height: 140 }}>
              <CardContent>
                <Stack direction="row" spacing={2}>
                  <Skeleton variant="circular" width={48} height={48} />
                  <Box sx={{ flex: 1 }}>
                    <Skeleton variant="text" height={32} width="60%" />
                    <Skeleton variant="text" height={20} width="80%" />
                  </Box>
                </Stack>
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>
    );
  }

  const totalFailures = stats.active_failures + stats.resolved_failures;
  const criticalPercentage = totalFailures > 0 ? (stats.critical_failures / totalFailures) * 100 : 0;
  const highPercentage = totalFailures > 0 ? (stats.high_failures / totalFailures) * 100 : 0;
  const mediumPercentage = totalFailures > 0 ? (stats.medium_failures / totalFailures) * 100 : 0;
  const lowPercentage = totalFailures > 0 ? (stats.low_failures / totalFailures) * 100 : 0;
  const retryPercentage = stats.active_failures > 0 ? (stats.ready_for_retry / stats.active_failures) * 100 : 0;

  return (
    <Box sx={{ mb: 4 }}>
      <Typography
        variant="h6"
        sx={{
          fontWeight: 600,
          color: modernTokens.colors.neutral[900],
          mb: 3,
        }}
      >
        Source Failure Statistics
      </Typography>

      <Grid container spacing={3}>
        {/* Total Active Failures */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="Active Failures"
            value={stats.active_failures}
            icon={ErrorIcon}
            color={modernTokens.colors.error[500]}
            bgColor={modernTokens.colors.error[50]}
            description="Requiring attention"
          />
        </Grid>

        {/* Critical Failures */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="Critical"
            value={stats.critical_failures}
            icon={ErrorIcon}
            color={modernTokens.colors.error[600]}
            bgColor={modernTokens.colors.error[50]}
            percentage={criticalPercentage}
            description="Immediate action needed"
          />
        </Grid>

        {/* High Priority Failures */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="High Priority"
            value={stats.high_failures}
            icon={WarningIcon}
            color={modernTokens.colors.warning[600]}
            bgColor={modernTokens.colors.warning[50]}
            percentage={highPercentage}
            description="Important issues"
          />
        </Grid>

        {/* Medium Priority Failures */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="Medium Priority"
            value={stats.medium_failures}
            icon={InfoIcon}
            color={modernTokens.colors.warning[500]}
            bgColor={modernTokens.colors.warning[50]}
            percentage={mediumPercentage}
            description="Moderate issues"
          />
        </Grid>

        {/* Low Priority Failures */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="Low Priority"
            value={stats.low_failures}
            icon={InfoIcon}
            color={modernTokens.colors.info[500]}
            bgColor={modernTokens.colors.info[50]}
            percentage={lowPercentage}
            description="Minor issues"
          />
        </Grid>

        {/* Ready for Retry */}
        <Grid item xs={12} sm={6} md={4} lg={2}>
          <StatCard
            title="Ready for Retry"
            value={stats.ready_for_retry}
            icon={RefreshIcon}
            color={modernTokens.colors.primary[500]}
            bgColor={modernTokens.colors.primary[50]}
            percentage={retryPercentage}
            description="Can be retried now"
          />
        </Grid>
      </Grid>

      {/* Summary Row */}
      <Grid container spacing={3} sx={{ mt: 2 }}>
        <Grid item xs={12} sm={6} md={4}>
          <StatCard
            title="Resolved Failures"
            value={stats.resolved_failures}
            icon={CheckCircleIcon}
            color={modernTokens.colors.success[500]}
            bgColor={modernTokens.colors.success[50]}
            description="Successfully resolved"
          />
        </Grid>

        <Grid item xs={12} sm={6} md={4}>
          <StatCard
            title="Excluded Resources"
            value={stats.excluded_resources}
            icon={BlockIcon}
            color={modernTokens.colors.neutral[500]}
            bgColor={modernTokens.colors.neutral[50]}
            description="Manually excluded resources"
          />
        </Grid>

        <Grid item xs={12} sm={6} md={4}>
          <Card
            sx={{
              height: '100%',
              background: `linear-gradient(135deg, ${modernTokens.colors.primary[50]} 0%, ${modernTokens.colors.primary[25]} 100%)`,
              border: `1px solid ${modernTokens.colors.primary[200]}`,
              borderRadius: 3,
            }}
          >
            <CardContent>
              <Stack spacing={2}>
                <Typography
                  variant="h6"
                  sx={{
                    fontWeight: 600,
                    color: modernTokens.colors.neutral[900],
                  }}
                >
                  Success Rate
                </Typography>
                
                <Box>
                  {totalFailures > 0 ? (
                    <>
                      <Typography
                        variant="h4"
                        sx={{
                          fontWeight: 700,
                          color: modernTokens.colors.primary[600],
                          mb: 1,
                        }}
                      >
                        {((stats.resolved_failures / totalFailures) * 100).toFixed(1)}%
                      </Typography>
                      <LinearProgress
                        variant="determinate"
                        value={(stats.resolved_failures / totalFailures) * 100}
                        sx={{
                          height: 8,
                          borderRadius: 4,
                          backgroundColor: modernTokens.colors.primary[100],
                          '& .MuiLinearProgress-bar': {
                            borderRadius: 4,
                            backgroundColor: modernTokens.colors.primary[500],
                          },
                        }}
                      />
                      <Typography
                        variant="caption"
                        sx={{
                          color: modernTokens.colors.neutral[600],
                          mt: 1,
                          display: 'block',
                        }}
                      >
                        {stats.resolved_failures} of {totalFailures} failures resolved
                      </Typography>
                    </>
                  ) : (
                    <Typography
                      variant="h4"
                      sx={{
                        fontWeight: 700,
                        color: modernTokens.colors.success[600],
                      }}
                    >
                      100%
                    </Typography>
                  )}
                </Box>
              </Stack>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Source Type Breakdown */}
      {stats.failures_by_source_type && Object.keys(stats.failures_by_source_type).length > 0 && (
        <>
          <Typography
            variant="h6"
            sx={{
              fontWeight: 600,
              color: modernTokens.colors.neutral[900],
              mt: 4,
              mb: 3,
            }}
          >
            Failures by Source Type
          </Typography>
          
          <Grid container spacing={3}>
            {Object.entries(stats.failures_by_source_type).map(([sourceType, count]) => {
              const sourceConfig = {
                webdav: { 
                  label: 'WebDAV', 
                  icon: '🌐', 
                  color: modernTokens.colors.blue[500],
                  bgColor: modernTokens.colors.blue[50]
                },
                s3: { 
                  label: 'Amazon S3', 
                  icon: '☁️', 
                  color: modernTokens.colors.orange[500],
                  bgColor: modernTokens.colors.orange[50]
                },
                local_folder: { 
                  label: 'Local Folder', 
                  icon: '📁', 
                  color: modernTokens.colors.green[500],
                  bgColor: modernTokens.colors.green[50]
                }
              }[sourceType] || { 
                label: sourceType, 
                icon: '❓', 
                color: modernTokens.colors.neutral[500],
                bgColor: modernTokens.colors.neutral[50]
              };

              return (
                <Grid item xs={12} sm={6} md={4} key={sourceType}>
                  <Card
                    sx={{
                      height: '100%',
                      backgroundColor: sourceConfig.bgColor,
                      border: `1px solid ${sourceConfig.color}20`,
                      borderRadius: 3,
                    }}
                  >
                    <CardContent>
                      <Stack direction="row" spacing={2}>
                        <Box
                          sx={{
                            fontSize: 32,
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                          }}
                        >
                          {sourceConfig.icon}
                        </Box>
                        <Box sx={{ flex: 1 }}>
                          <Typography
                            variant="h4"
                            sx={{
                              fontWeight: 700,
                              color: sourceConfig.color,
                              mb: 0.5,
                            }}
                          >
                            {count}
                          </Typography>
                          <Typography
                            variant="body2"
                            sx={{
                              color: modernTokens.colors.neutral[600],
                              fontWeight: 500,
                            }}
                          >
                            {sourceConfig.label}
                          </Typography>
                          <Typography
                            variant="caption"
                            sx={{
                              color: modernTokens.colors.neutral[500],
                              display: 'block',
                              mt: 0.5,
                            }}
                          >
                            {totalFailures > 0 ? `${((count / totalFailures) * 100).toFixed(1)}% of total` : '0% of total'}
                          </Typography>
                        </Box>
                      </Stack>
                    </CardContent>
                  </Card>
                </Grid>
              );
            })}
          </Grid>
        </>
      )}
    </Box>
  );
};

export default StatsDashboard;