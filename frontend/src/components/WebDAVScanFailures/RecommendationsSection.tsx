import React from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Stack,
  Chip,
  Alert,
  Button,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Divider,
  Link,
} from '@mui/material';
import {
  Lightbulb as LightbulbIcon,
  Schedule as ScheduleIcon,
  Folder as FolderIcon,
  Security as SecurityIcon,
  NetworkCheck as NetworkIcon,
  Settings as SettingsIcon,
  Speed as SpeedIcon,
  Warning as WarningIcon,
  Info as InfoIcon,
  OpenInNew as ExternalLinkIcon,
} from '@mui/icons-material';

import { WebDAVScanFailure, WebDAVScanFailureType } from '../../services/api';
import { modernTokens } from '../../theme';

interface RecommendationsSectionProps {
  failures: WebDAVScanFailure[];
}

interface RecommendationInfo {
  icon: React.ElementType;
  title: string;
  description: string;
  actions: string[];
  learnMoreUrl?: string;
  severity: 'info' | 'warning' | 'error';
}

const getRecommendationsForFailureType = (type: WebDAVScanFailureType): RecommendationInfo => {
  const recommendations: Record<WebDAVScanFailureType, RecommendationInfo> = {
    timeout: {
      icon: ScheduleIcon,
      title: 'Timeout Issues',
      description: 'Directories are taking too long to scan. This often indicates large directories or slow server response.',
      actions: [
        'Consider organizing files into smaller subdirectories',
        'Check your network connection speed',
        'Verify the WebDAV server performance',
        'Try scanning during off-peak hours',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#timeout-issues',
      severity: 'warning',
    },
    path_too_long: {
      icon: FolderIcon,
      title: 'Path Length Limits',
      description: 'File paths are exceeding the maximum allowed length (typically 260 characters on Windows, 4096 on Unix).',
      actions: [
        'Shorten directory and file names',
        'Reduce nesting depth of folders',
        'Move files to a shorter base path',
        'Consider using symbolic links for deep structures',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#path-length',
      severity: 'error',
    },
    permission_denied: {
      icon: SecurityIcon,
      title: 'Permission Issues',
      description: 'The WebDAV client does not have sufficient permissions to access these directories.',
      actions: [
        'Verify your WebDAV username and password',
        'Check directory permissions on the server',
        'Ensure the user has read access to all subdirectories',
        'Contact your system administrator if needed',
      ],
      learnMoreUrl: '/docs/webdav-setup#permissions',
      severity: 'error',
    },
    invalid_characters: {
      icon: WarningIcon,
      title: 'Invalid Characters',
      description: 'File or directory names contain characters that are not supported by the file system or WebDAV protocol.',
      actions: [
        'Remove or replace special characters in file names',
        'Avoid characters like: < > : " | ? * \\',
        'Use ASCII characters when possible',
        'Rename files with Unicode characters if causing issues',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#invalid-characters',
      severity: 'warning',
    },
    network_error: {
      icon: NetworkIcon,
      title: 'Network Connectivity',
      description: 'Unable to establish a stable connection to the WebDAV server.',
      actions: [
        'Check your internet connection',
        'Verify the WebDAV server URL is correct',
        'Test connectivity with other WebDAV clients',
        'Check firewall settings',
        'Try using a different network',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#network-issues',
      severity: 'error',
    },
    server_error: {
      icon: SettingsIcon,
      title: 'Server Issues',
      description: 'The WebDAV server returned an error. This may be temporary or indicate server configuration issues.',
      actions: [
        'Wait and retry - server issues are often temporary',
        'Check server logs for detailed error information',
        'Verify server configuration and resources',
        'Contact your WebDAV server administrator',
        'Try accessing the server with other clients',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#server-errors',
      severity: 'warning',
    },
    xml_parse_error: {
      icon: WarningIcon,
      title: 'Protocol Issues',
      description: 'Unable to parse the server response. This may indicate WebDAV protocol compatibility issues.',
      actions: [
        'Verify the server supports WebDAV protocol',
        'Check if the server returns valid XML responses',
        'Try connecting with different WebDAV client settings',
        'Update the server software if possible',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#protocol-issues',
      severity: 'warning',
    },
    too_many_items: {
      icon: SpeedIcon,
      title: 'Large Directory Optimization',
      description: 'Directories contain too many files, causing performance issues and potential timeouts.',
      actions: [
        'Organize files into multiple subdirectories',
        'Archive old files to reduce directory size',
        'Use date-based or category-based folder structures',
        'Consider excluding very large directories temporarily',
      ],
      learnMoreUrl: '/docs/webdav-optimization#large-directories',
      severity: 'warning',
    },
    depth_limit: {
      icon: FolderIcon,
      title: 'Directory Depth Limits',
      description: 'Directory nesting is too deep, exceeding system or protocol limits.',
      actions: [
        'Flatten the directory structure',
        'Move deeply nested files to shallower locations',
        'Reorganize the folder hierarchy',
        'Use shorter path names at each level',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#depth-limits',
      severity: 'warning',
    },
    size_limit: {
      icon: SpeedIcon,
      title: 'Size Limitations',
      description: 'Files or directories are too large for the current configuration.',
      actions: [
        'Check file size limits on the WebDAV server',
        'Split large files into smaller parts',
        'Exclude very large files from synchronization',
        'Increase server limits if possible',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#size-limits',
      severity: 'warning',
    },
    unknown: {
      icon: InfoIcon,
      title: 'Unknown Issues',
      description: 'An unclassified error occurred. This may require manual investigation.',
      actions: [
        'Check the detailed error message for clues',
        'Try the operation again later',
        'Contact support with the full error details',
        'Check server and client logs',
      ],
      learnMoreUrl: '/docs/webdav-troubleshooting#general',
      severity: 'info',
    },
  };

  return recommendations[type];
};

const RecommendationsSection: React.FC<RecommendationsSectionProps> = ({ failures }) => {
  // Group failures by type and get unique types
  const failureTypeStats = failures.reduce((acc, failure) => {
    if (!failure.resolved && !failure.user_excluded) {
      acc[failure.failure_type] = (acc[failure.failure_type] || 0) + 1;
    }
    return acc;
  }, {} as Record<WebDAVScanFailureType, number>);

  const activeFailureTypes = Object.keys(failureTypeStats) as WebDAVScanFailureType[];

  if (activeFailureTypes.length === 0) {
    return null;
  }

  // Sort by frequency (most common issues first)
  const sortedFailureTypes = activeFailureTypes.sort(
    (a, b) => failureTypeStats[b] - failureTypeStats[a]
  );

  return (
    <Card
      sx={{
        backgroundColor: modernTokens.colors.primary[50],
        border: `1px solid ${modernTokens.colors.primary[200]}`,
        borderRadius: 3,
      }}
    >
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 3 }}>
          <LightbulbIcon sx={{ color: modernTokens.colors.primary[600], fontSize: 24 }} />
          <Typography
            variant="h6"
            sx={{
              fontWeight: 600,
              color: modernTokens.colors.primary[700],
            }}
          >
            Recommendations & Solutions
          </Typography>
        </Box>

        <Typography
          variant="body2"
          sx={{
            color: modernTokens.colors.neutral[600],
            mb: 3,
          }}
        >
          Based on your current scan failures, here are targeted recommendations to resolve common issues:
        </Typography>

        <Stack spacing={3}>
          {sortedFailureTypes.map((failureType, index) => {
            const recommendation = getRecommendationsForFailureType(failureType);
            const Icon = recommendation.icon;
            const count = failureTypeStats[failureType];

            return (
              <Box key={failureType}>
                {index > 0 && <Divider sx={{ my: 2 }} />}
                
                <Card
                  variant="outlined"
                  sx={{
                    backgroundColor: modernTokens.colors.neutral[0],
                    border: `1px solid ${modernTokens.colors.neutral[200]}`,
                  }}
                >
                  <CardContent>
                    <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 2, mb: 2 }}>
                      <Icon 
                        sx={{ 
                          color: recommendation.severity === 'error' 
                            ? modernTokens.colors.error[500]
                            : recommendation.severity === 'warning'
                            ? modernTokens.colors.warning[500]
                            : modernTokens.colors.info[500],
                          fontSize: 24,
                          mt: 0.5,
                        }} 
                      />
                      
                      <Box sx={{ flex: 1 }}>
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 1 }}>
                          <Typography
                            variant="h6"
                            sx={{
                              fontWeight: 600,
                              color: modernTokens.colors.neutral[900],
                            }}
                          >
                            {recommendation.title}
                          </Typography>
                          
                          <Chip
                            label={`${count} ${count === 1 ? 'failure' : 'failures'}`}
                            size="small"
                            sx={{
                              backgroundColor: recommendation.severity === 'error' 
                                ? modernTokens.colors.error[100]
                                : recommendation.severity === 'warning'
                                ? modernTokens.colors.warning[100]
                                : modernTokens.colors.info[100],
                              color: recommendation.severity === 'error' 
                                ? modernTokens.colors.error[700]
                                : recommendation.severity === 'warning'
                                ? modernTokens.colors.warning[700]
                                : modernTokens.colors.info[700],
                            }}
                          />
                        </Box>

                        <Typography
                          variant="body2"
                          sx={{
                            color: modernTokens.colors.neutral[600],
                            mb: 2,
                            lineHeight: 1.6,
                          }}
                        >
                          {recommendation.description}
                        </Typography>

                        <Typography
                          variant="subtitle2"
                          sx={{
                            fontWeight: 600,
                            color: modernTokens.colors.neutral[800],
                            mb: 1,
                          }}
                        >
                          Recommended Actions:
                        </Typography>

                        <List dense sx={{ py: 0 }}>
                          {recommendation.actions.map((action, actionIndex) => (
                            <ListItem key={actionIndex} sx={{ py: 0.5, px: 0 }}>
                              <ListItemIcon sx={{ minWidth: 32 }}>
                                <Box
                                  sx={{
                                    width: 6,
                                    height: 6,
                                    borderRadius: '50%',
                                    backgroundColor: modernTokens.colors.primary[500],
                                  }}
                                />
                              </ListItemIcon>
                              <ListItemText
                                primary={action}
                                primaryTypographyProps={{
                                  variant: 'body2',
                                  sx: { color: modernTokens.colors.neutral[700] },
                                }}
                              />
                            </ListItem>
                          ))}
                        </List>

                        {recommendation.learnMoreUrl && (
                          <Box sx={{ mt: 2 }}>
                            <Link
                              href={recommendation.learnMoreUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              sx={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: 0.5,
                                color: modernTokens.colors.primary[600],
                                textDecoration: 'none',
                                fontSize: '0.875rem',
                                '&:hover': {
                                  textDecoration: 'underline',
                                },
                              }}
                            >
                              Learn more about this issue
                              <ExternalLinkIcon sx={{ fontSize: 16 }} />
                            </Link>
                          </Box>
                        )}
                      </Box>
                    </Box>
                  </CardContent>
                </Card>
              </Box>
            );
          })}
        </Stack>

        {/* General Tips */}
        <Box sx={{ mt: 4 }}>
          <Alert
            severity="info"
            sx={{
              backgroundColor: modernTokens.colors.info[50],
              borderColor: modernTokens.colors.info[200],
              '& .MuiAlert-message': {
                width: '100%',
              },
            }}
          >
            <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1 }}>
              General Troubleshooting Tips:
            </Typography>
            <List dense sx={{ py: 0 }}>
              <ListItem sx={{ py: 0, px: 0 }}>
                <ListItemText
                  primary="Most issues resolve automatically after addressing the underlying cause"
                  primaryTypographyProps={{ variant: 'body2' }}
                />
              </ListItem>
              <ListItem sx={{ py: 0, px: 0 }}>
                <ListItemText
                  primary="Use the retry function after making changes to test the fix"
                  primaryTypographyProps={{ variant: 'body2' }}
                />
              </ListItem>
              <ListItem sx={{ py: 0, px: 0 }}>
                <ListItemText
                  primary="Exclude problematic directories temporarily while working on solutions"
                  primaryTypographyProps={{ variant: 'body2' }}
                />
              </ListItem>
              <ListItem sx={{ py: 0, px: 0 }}>
                <ListItemText
                  primary="Monitor the statistics dashboard to track improvement over time"
                  primaryTypographyProps={{ variant: 'body2' }}
                />
              </ListItem>
            </List>
          </Alert>
        </Box>
      </CardContent>
    </Card>
  );
};

export default RecommendationsSection;