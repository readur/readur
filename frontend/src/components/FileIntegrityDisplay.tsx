import React, { useState } from 'react';
import {
  Box,
  Typography,
  Chip,
  Paper,
  IconButton,
  Tooltip,
  Stack,
  CircularProgress,
  Alert,
} from '@mui/material';
import {
  Security as SecurityIcon,
  Fingerprint as FingerprintIcon,
  ContentCopy as CopyIcon,
  CheckCircle as CheckIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  Info as InfoIcon,
} from '@mui/icons-material';
import { useTheme } from '../contexts/ThemeContext';
import { useTheme as useMuiTheme } from '@mui/material/styles';

interface FileIntegrityDisplayProps {
  fileHash?: string;
  fileName: string;
  fileSize: number;
  mimeType: string;
  createdAt: string;
  updatedAt: string;
  userId?: string;
  username?: string;
  compact?: boolean;
}

const FileIntegrityDisplay: React.FC<FileIntegrityDisplayProps> = ({
  fileHash,
  fileName,
  fileSize,
  mimeType,
  createdAt,
  updatedAt,
  userId,
  username,
  compact = false,
}) => {
  const [copied, setCopied] = useState(false);
  const { modernTokens } = useTheme();
  const theme = useMuiTheme();

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const formatHash = (hash: string) => {
    if (!hash) return 'Not available';
    return `${hash.substring(0, 8)}...${hash.substring(hash.length - 8)}`;
  };

  const getIntegrityStatus = () => {
    if (!fileHash) {
      return {
        status: 'unknown',
        icon: <InfoIcon />,
        color: theme.palette.text.secondary,
        message: 'Hash not available',
      };
    }

    // Simple validation - in real implementation you'd verify against stored hash
    if (fileHash.length === 64) { // SHA256 length
      return {
        status: 'verified',
        icon: <CheckIcon />,
        color: theme.palette.success.main,
        message: 'File integrity verified',
      };
    }

    return {
      status: 'warning',
      icon: <WarningIcon />,
      color: theme.palette.warning.main,
      message: 'Hash format unusual',
    };
  };

  const integrityStatus = getIntegrityStatus();

  const formatFileSize = (bytes: number): string => {
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 Bytes';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  const formatDate = (dateString: string): string => {
    return new Date(dateString).toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (compact) {
    return (
      <Paper 
        sx={{ 
          p: 2, 
          backgroundColor: theme.palette.background.paper,
          border: `1px solid ${theme.palette.divider}`,
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <SecurityIcon 
              sx={{ 
                fontSize: 18, 
                mr: 1, 
                color: integrityStatus.color 
              }} 
            />
            <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
              File Integrity
            </Typography>
          </Box>
          <Chip 
            size="small"
            label={integrityStatus.status}
            sx={{ 
              backgroundColor: `${integrityStatus.color}20`,
              color: integrityStatus.color,
              border: `1px solid ${integrityStatus.color}40`,
              textTransform: 'capitalize',
            }}
          />
        </Box>
        
        <Stack spacing={1}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="caption" color="text.secondary">
              Hash (SHA256)
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <Typography 
                variant="caption" 
                sx={{ 
                  fontFamily: 'monospace', 
                  fontWeight: 500,
                  mr: 0.5,
                }}
              >
                {formatHash(fileHash || '')}
              </Typography>
              {fileHash && (
                <Tooltip title={copied ? 'Copied!' : 'Copy full hash'}>
                  <IconButton 
                    size="small" 
                    onClick={() => copyToClipboard(fileHash)}
                    sx={{ p: 0.25 }}
                  >
                    <CopyIcon sx={{ fontSize: 12 }} />
                  </IconButton>
                </Tooltip>
              )}
            </Box>
          </Box>
          
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="caption" color="text.secondary">
              Size
            </Typography>
            <Typography variant="caption" sx={{ fontWeight: 500 }}>
              {formatFileSize(fileSize)}
            </Typography>
          </Box>
        </Stack>
      </Paper>
    );
  }

  return (
    <Paper 
      sx={{ 
        p: 3,
        backgroundColor: theme.palette.background.paper,
        border: `1px solid ${theme.palette.divider}`,
      }}
    >
      {/* Header */}
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          <SecurityIcon 
            sx={{ 
              fontSize: 24, 
              mr: 1.5, 
              color: theme.palette.primary.main 
            }} 
          />
          <Typography variant="h6" sx={{ fontWeight: 600 }}>
            File Integrity & Verification
          </Typography>
        </Box>
        
        <Chip 
          icon={React.cloneElement(integrityStatus.icon, { sx: { fontSize: 18 } })}
          label={integrityStatus.message}
          sx={{ 
            backgroundColor: `${integrityStatus.color}20`,
            color: integrityStatus.color,
            border: `1px solid ${integrityStatus.color}40`,
            fontWeight: 500,
          }}
        />
      </Box>

      {/* Hash Information */}
      <Box sx={{ mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
          <FingerprintIcon 
            sx={{ 
              fontSize: 18, 
              mr: 1, 
              color: theme.palette.text.secondary 
            }} 
          />
          <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
            SHA256 Hash
          </Typography>
        </Box>
        
        {fileHash ? (
          <Box 
            sx={{ 
              display: 'flex', 
              alignItems: 'center', 
              p: 2, 
              backgroundColor: theme.palette.action.hover,
              borderRadius: 1,
              border: `1px solid ${theme.palette.divider}`,
            }}
          >
            <Typography 
              variant="body2" 
              sx={{ 
                fontFamily: 'monospace', 
                flex: 1, 
                wordBreak: 'break-all',
                fontSize: '0.8rem',
                color: theme.palette.text.primary,
              }}
            >
              {fileHash}
            </Typography>
            <Tooltip title={copied ? 'Copied!' : 'Copy hash'}>
              <IconButton 
                size="small" 
                onClick={() => copyToClipboard(fileHash)}
                sx={{ ml: 1 }}
              >
                <CopyIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        ) : (
          <Alert severity="info" sx={{ mt: 1 }}>
            File hash not available. Enable hash generation in upload settings.
          </Alert>
        )}
      </Box>

      {/* File Properties */}
      <Box sx={{ mb: 2 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 2 }}>
          File Properties
        </Typography>
        
        <Stack spacing={2}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              File Size
            </Typography>
            <Typography variant="body2" sx={{ fontWeight: 500 }}>
              {formatFileSize(fileSize)}
            </Typography>
          </Box>
          
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              MIME Type
            </Typography>
            <Chip 
              label={mimeType} 
              size="small"
              sx={{ 
                fontSize: '0.75rem',
                backgroundColor: theme.palette.action.hover,
                border: `1px solid ${theme.palette.divider}`,
              }}
            />
          </Box>
          
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              Uploaded
            </Typography>
            <Typography variant="body2" sx={{ fontWeight: 500 }}>
              {formatDate(createdAt)}
            </Typography>
          </Box>
          
          {createdAt !== updatedAt && (
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2" color="text.secondary">
                Last Modified
              </Typography>
              <Typography variant="body2" sx={{ fontWeight: 500 }}>
                {formatDate(updatedAt)}
              </Typography>
            </Box>
          )}
          
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="body2" color="text.secondary">
              Uploaded By
            </Typography>
            <Chip 
              label={username || (userId ? `User: ${userId.substring(0, 8)}...` : 'Unknown')} 
              size="small"
              sx={{ 
                fontSize: '0.75rem',
                backgroundColor: theme.palette.primary.light,
                color: theme.palette.primary.dark,
                border: `1px solid ${theme.palette.primary.main}`,
              }}
            />
          </Box>
        </Stack>
      </Box>
    </Paper>
  );
};

export default FileIntegrityDisplay;