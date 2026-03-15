import React, { useState } from 'react';
import {
  Box,
  Typography,
  Chip,
  Stack,
  IconButton,
  Tooltip,
  Alert,
  Collapse,
} from '@mui/material';
import {
  ContentCopy as CopyIcon,
  ExpandMore as ExpandMoreIcon,
} from '@mui/icons-material';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { type Document } from '../../services/api';

interface DetailsTabProps {
  document: Document;
  formatFileSize: (bytes: number) => string;
  formatDate: (dateString: string) => string;
}

const MetadataRow: React.FC<{
  label: string;
  value: React.ReactNode;
}> = ({ label, value }) => (
  <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', py: 0.75 }}>
    <Typography variant="body2" color="text.secondary">
      {label}
    </Typography>
    <Box sx={{ textAlign: 'right', maxWidth: '60%' }}>
      {typeof value === 'string' ? (
        <Typography variant="body2" sx={{ fontWeight: 500 }}>
          {value}
        </Typography>
      ) : (
        value
      )}
    </Box>
  </Box>
);

interface SectionProps {
  title: string;
  defaultExpanded?: boolean;
  children: React.ReactNode;
}

const Section: React.FC<SectionProps> = ({ title, defaultExpanded = false, children }) => {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const theme = useMuiTheme();

  return (
    <Box sx={{ borderBottom: `1px solid ${theme.palette.divider}` }}>
      <Box
        onClick={() => setExpanded(!expanded)}
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          py: 1.5,
          cursor: 'pointer',
          '&:hover': { backgroundColor: theme.palette.action.hover },
          px: 1,
          borderRadius: 0.5,
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {title}
        </Typography>
        <ExpandMoreIcon
          sx={{
            fontSize: 20,
            color: theme.palette.text.secondary,
            transform: expanded ? 'rotate(180deg)' : 'none',
            transition: 'transform 0.2s',
          }}
        />
      </Box>
      <Collapse in={expanded}>
        <Box sx={{ px: 1, pb: 2 }}>
          {children}
        </Box>
      </Collapse>
    </Box>
  );
};

const DetailsTab: React.FC<DetailsTabProps> = ({
  document,
  formatFileSize,
  formatDate,
}) => {
  const theme = useMuiTheme();
  const [copied, setCopied] = useState(false);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const formatHash = (hash: string) => {
    if (!hash) return 'Not available';
    return `${hash.substring(0, 8)}...${hash.substring(hash.length - 8)}`;
  };

  return (
    <Box>
      {/* File Properties - expanded by default */}
      <Section title="File Properties" defaultExpanded>
        <Stack spacing={0}>
          <MetadataRow label="Filename" value={document.original_filename} />
          <MetadataRow label="MIME Type" value={
            <Chip
              label={document.mime_type}
              size="small"
              sx={{
                fontSize: '0.75rem',
                backgroundColor: theme.palette.action.hover,
                border: `1px solid ${theme.palette.divider}`,
              }}
            />
          } />
          <MetadataRow label="File Size" value={formatFileSize(document.file_size)} />
          <MetadataRow label="Uploaded" value={formatDate(document.created_at)} />
          {document.created_at !== document.updated_at && (
            <MetadataRow label="Last Modified" value={formatDate(document.updated_at)} />
          )}
          <MetadataRow label="Uploaded By" value={
            <Chip
              label={document.username || (document.user_id ? `User: ${document.user_id.substring(0, 8)}...` : 'Unknown')}
              size="small"
              sx={{
                fontSize: '0.75rem',
                backgroundColor: theme.palette.primary.light,
                color: theme.palette.primary.dark,
              }}
            />
          } />
          {document.file_owner && (
            <MetadataRow label="Owner" value={
              <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.8rem', fontWeight: 500 }}>
                {document.file_owner}
              </Typography>
            } />
          )}
        </Stack>
      </Section>

      {/* File Integrity / SHA256 - expanded by default */}
      <Section title="File Integrity" defaultExpanded>
        {document.file_hash ? (
          <>
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                p: 1.5,
                backgroundColor: theme.palette.action.hover,
                borderRadius: 1,
                border: `1px solid ${theme.palette.divider}`,
                mb: 1,
              }}
            >
              <Typography
                variant="body2"
                sx={{
                  fontFamily: 'monospace',
                  flex: 1,
                  wordBreak: 'break-all',
                  fontSize: '0.8rem',
                }}
              >
                {document.file_hash}
              </Typography>
              <Tooltip title={copied ? 'Copied!' : 'Copy hash'}>
                <IconButton size="small" onClick={() => copyToClipboard(document.file_hash!)} sx={{ ml: 1 }}>
                  <CopyIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Box>
            <Typography variant="caption" color="text.secondary">SHA256</Typography>
            {document.file_hash.length === 64 && (
              <Chip
                label="Verified"
                size="small"
                color="success"
                sx={{ ml: 1, fontSize: '0.7rem', height: 20 }}
              />
            )}
          </>
        ) : (
          <Alert severity="info" sx={{ py: 0.5 }}>
            File hash not available.
          </Alert>
        )}
      </Section>

      {/* Source Information - collapsed */}
      {(document.source_type || document.source_path || document.source_id) && (
        <Section title="Source Information">
          <Stack spacing={0}>
            {document.source_type && (
              <MetadataRow label="Source Type" value={
                <Chip
                  label={document.source_type.replace('_', ' ').toUpperCase()}
                  size="small"
                  sx={{
                    fontSize: '0.75rem',
                    backgroundColor: theme.palette.info.light,
                    color: theme.palette.info.dark,
                  }}
                />
              } />
            )}
            {document.source_path && (
              <MetadataRow label="Source Path" value={
                <Typography
                  variant="body2"
                  sx={{
                    fontFamily: 'monospace',
                    fontSize: '0.8rem',
                    fontWeight: 500,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                  title={document.source_path}
                >
                  {document.source_path}
                </Typography>
              } />
            )}
            {document.source_id && (
              <MetadataRow label="Source ID" value={
                <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                  {document.source_id}
                </Typography>
              } />
            )}
            {document.file_group && (
              <MetadataRow label="File Group" value={
                <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                  {document.file_group}
                </Typography>
              } />
            )}
            {document.file_permissions != null && (
              <MetadataRow label="Permissions" value={
                <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                  {document.file_permissions.toString(8)} ({document.file_permissions})
                </Typography>
              } />
            )}
          </Stack>
        </Section>
      )}

      {/* Original Timestamps - collapsed */}
      {(document.original_created_at || document.original_modified_at) && (
        <Section title="Original Timestamps">
          <Stack spacing={0}>
            {document.original_created_at && (
              <MetadataRow label="Original Created" value={formatDate(document.original_created_at)} />
            )}
            {document.original_modified_at && (
              <MetadataRow label="Original Modified" value={formatDate(document.original_modified_at)} />
            )}
          </Stack>
        </Section>
      )}

      {/* Source Metadata - collapsed */}
      {document.source_metadata && Object.keys(document.source_metadata).length > 0 && (
        <Section title="Source Metadata">
          <Stack spacing={0}>
            {Object.entries(document.source_metadata).map(([key, value]) => {
              if (value === null || value === undefined || typeof value === 'object') return null;
              const formattedKey = key
                .replace(/_/g, ' ')
                .replace(/([A-Z])/g, ' $1')
                .replace(/^./, str => str.toUpperCase())
                .trim();
              const formattedValue = typeof value === 'boolean'
                ? (value ? 'Yes' : 'No')
                : String(value);
              return (
                <MetadataRow key={key} label={formattedKey} value={formattedValue} />
              );
            }).filter(Boolean)}
          </Stack>
        </Section>
      )}
    </Box>
  );
};

export default DetailsTab;
