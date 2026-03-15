import React from 'react';
import {
  Box,
  Button,
  CircularProgress,
  Typography,
} from '@mui/material';
import {
  PhotoFilter as ProcessedImageIcon,
} from '@mui/icons-material';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import DocumentViewer from '../DocumentViewer';
import { type Document } from '../../services/api';

interface PreviewTabProps {
  document: Document;
  processedImageLoading: boolean;
  onViewProcessedImage: () => void;
  t: (key: string, options?: any) => string;
}

const PreviewTab: React.FC<PreviewTabProps> = ({
  document,
  processedImageLoading,
  onViewProcessedImage,
  t,
}) => {
  const theme = useMuiTheme();

  return (
    <Box>
      <Box
        sx={{
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 1,
          overflow: 'hidden',
          minHeight: 400,
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <DocumentViewer
          documentId={document.id}
          filename={document.original_filename}
          mimeType={document.mime_type}
        />
      </Box>

      {document.mime_type?.includes('image') && (
        <Box sx={{ mt: 2 }}>
          <Button
            variant="outlined"
            size="small"
            startIcon={processedImageLoading ? <CircularProgress size={16} /> : <ProcessedImageIcon />}
            onClick={onViewProcessedImage}
            disabled={processedImageLoading}
            sx={{ textTransform: 'none' }}
          >
            {t('documentDetails.actions.viewProcessedImage')}
          </Button>
        </Box>
      )}
    </Box>
  );
};

export default PreviewTab;
