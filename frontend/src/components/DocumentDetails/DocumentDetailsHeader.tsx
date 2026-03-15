import React from 'react';
import {
  Box,
  Typography,
  Button,
  Chip,
  Stack,
  IconButton,
  Tooltip,
  CircularProgress,
} from '@mui/material';
import {
  ArrowBack as BackIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  Delete as DeleteIcon,
  Refresh as RefreshIcon,
  Edit as EditIcon,
  Schedule as ScheduleIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
} from '@mui/icons-material';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { type Document } from '../../services/api';
import { type LabelData } from '../Labels/Label';

interface DocumentDetailsHeaderProps {
  document: Document;
  documentLabels: LabelData[];
  deleting: boolean;
  retryingOcr: boolean;
  onBack: () => void;
  onDownload: () => void;
  onDelete: () => void;
  onRetryOcr: () => void;
  onEditLabels: () => void;
  formatFileSize: (bytes: number) => string;
  formatDate: (dateString: string) => string;
  t: (key: string, options?: any) => string;
}

const DocumentDetailsHeader: React.FC<DocumentDetailsHeaderProps> = ({
  document,
  documentLabels,
  deleting,
  retryingOcr,
  onBack,
  onDownload,
  onDelete,
  onRetryOcr,
  onEditLabels,
  formatFileSize,
  formatDate,
  t,
}) => {
  const theme = useMuiTheme();

  const getFileTypeLabel = (mimeType?: string): string => {
    if (mimeType?.includes('pdf')) return 'PDF';
    if (mimeType?.includes('image/png')) return 'PNG';
    if (mimeType?.includes('image/jpeg') || mimeType?.includes('image/jpg')) return 'JPEG';
    if (mimeType?.includes('image')) return 'Image';
    if (mimeType?.includes('text')) return 'Text';
    return 'Document';
  };

  const getOcrStatusChip = () => {
    const status = document.ocr_status;
    if (!status || status === 'pending') {
      return (
        <Chip
          icon={<ScheduleIcon sx={{ fontSize: 16 }} />}
          label="Pending OCR"
          size="small"
          sx={{
            backgroundColor: theme.palette.warning.light,
            color: theme.palette.warning.dark,
            fontWeight: 600,
          }}
        />
      );
    }
    if (status === 'processing') {
      return (
        <Chip
          icon={<CircularProgress size={14} sx={{ ml: 0.5 }} />}
          label="Processing..."
          size="small"
          sx={{
            backgroundColor: theme.palette.info.light,
            color: theme.palette.info.dark,
            fontWeight: 600,
          }}
        />
      );
    }
    if (status === 'completed') {
      return (
        <Chip
          icon={<CheckCircleIcon sx={{ fontSize: 16 }} />}
          label="OCR Complete"
          size="small"
          color="success"
          sx={{ fontWeight: 600 }}
        />
      );
    }
    if (status === 'failed') {
      return (
        <Stack direction="row" spacing={1} alignItems="center">
          <Chip
            icon={<ErrorIcon sx={{ fontSize: 16 }} />}
            label="OCR Failed"
            size="small"
            color="error"
            sx={{ fontWeight: 600 }}
          />
          <Button
            size="small"
            variant="outlined"
            color="error"
            onClick={onRetryOcr}
            disabled={retryingOcr}
            startIcon={retryingOcr ? <CircularProgress size={14} /> : <RefreshIcon sx={{ fontSize: 16 }} />}
            sx={{ textTransform: 'none', fontSize: '0.75rem' }}
          >
            Retry
          </Button>
        </Stack>
      );
    }
    return null;
  };

  const shortDate = (dateString: string): string => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  return (
    <Box sx={{ mb: 3 }}>
      <Button
        startIcon={<BackIcon />}
        onClick={onBack}
        sx={{
          mb: 2,
          color: theme.palette.text.secondary,
          textTransform: 'none',
          '&:hover': { backgroundColor: theme.palette.action.hover },
        }}
      >
        {t('documentDetails.actions.backToDocuments')}
      </Button>

      {/* Filename */}
      <Typography
        variant="h4"
        sx={{
          fontWeight: 700,
          color: theme.palette.text.primary,
          letterSpacing: '-0.02em',
          mb: 1,
        }}
      >
        {document.original_filename}
      </Typography>

      {/* Inline metadata + actions */}
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 1, mb: 1.5 }}>
        <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap" useFlexGap>
          <Typography variant="body2" color="text.secondary">
            {getFileTypeLabel(document.mime_type)} · {formatFileSize(document.file_size)} · Uploaded {shortDate(document.created_at)}
          </Typography>
          {getOcrStatusChip()}
        </Stack>

        <Stack direction="row" spacing={0.5}>
          <Tooltip title={t('documentDetails.actions.download')}>
            <IconButton onClick={onDownload} size="small">
              <DownloadIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title={t('documentDetails.actions.deleteDocument')}>
            <IconButton
              onClick={onDelete}
              disabled={deleting}
              size="small"
              sx={{ color: theme.palette.error.main }}
            >
              {deleting ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
            </IconButton>
          </Tooltip>
        </Stack>
      </Box>

      {/* Labels */}
      <Stack direction="row" spacing={0.5} alignItems="center" flexWrap="wrap" useFlexGap>
        {document.tags && document.tags.length > 0 && document.tags.map((tag, index) => (
          <Chip
            key={`tag-${index}`}
            label={tag}
            size="small"
            sx={{
              backgroundColor: theme.palette.primary.light,
              color: theme.palette.primary.dark,
              border: `1px solid ${theme.palette.primary.main}`,
              fontWeight: 500,
              fontSize: '0.75rem',
            }}
          />
        ))}
        {documentLabels.map((label) => (
          <Chip
            key={label.id}
            label={label.name}
            size="small"
            sx={{
              backgroundColor: label.background_color || `${label.color}20`,
              color: label.color,
              border: `1px solid ${label.color}`,
              fontWeight: 500,
              fontSize: '0.75rem',
            }}
          />
        ))}
        <Chip
          icon={<EditIcon sx={{ fontSize: 14 }} />}
          label="Edit"
          size="small"
          variant="outlined"
          onClick={onEditLabels}
          sx={{ fontSize: '0.75rem', cursor: 'pointer' }}
        />
      </Stack>

      {/* OCR Failed Alert */}
      {document.ocr_status === 'failed' && document.ocr_failure_reason && (
        <Box
          sx={{
            mt: 2,
            p: 1.5,
            borderRadius: 1,
            backgroundColor: theme.palette.error.light,
            border: `1px solid ${theme.palette.error.main}`,
          }}
        >
          <Typography variant="body2" color="error.dark">
            {document.ocr_failure_reason || document.ocr_error || 'OCR processing encountered an error.'}
          </Typography>
        </Box>
      )}
    </Box>
  );
};

export default DocumentDetailsHeader;
