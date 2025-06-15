import React, { useState, useEffect } from 'react';
import { Box } from '@mui/material';
import {
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  Description as DocIcon,
  TextSnippet as TextIcon,
} from '@mui/icons-material';
import { documentService } from '../services/api';

interface DocumentThumbnailProps {
  documentId: string;
  mimeType: string;
  size?: 'small' | 'medium' | 'large';
  fallbackIcon?: boolean;
}

const DocumentThumbnail: React.FC<DocumentThumbnailProps> = ({
  documentId,
  mimeType,
  size = 'medium',
  fallbackIcon = true,
}) => {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<boolean>(false);

  useEffect(() => {
    loadThumbnail();
    
    // Cleanup URL when component unmounts
    return () => {
      if (thumbnailUrl) {
        window.URL.revokeObjectURL(thumbnailUrl);
      }
    };
  }, [documentId]);

  const loadThumbnail = async (): Promise<void> => {
    try {
      setLoading(true);
      setError(false);
      
      const response = await documentService.getThumbnail(documentId);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      setThumbnailUrl(url);
    } catch (err) {
      setError(true);
    } finally {
      setLoading(false);
    }
  };

  const getFileIcon = (mimeType: string): React.ReactElement => {
    const iconProps = {
      sx: {
        fontSize: size === 'small' ? 24 : size === 'medium' ? 48 : 64,
        color: 'action.active',
      }
    };

    if (mimeType.includes('pdf')) return <PdfIcon {...iconProps} color="error" />;
    if (mimeType.includes('image')) return <ImageIcon {...iconProps} color="primary" />;
    if (mimeType.includes('text')) return <TextIcon {...iconProps} color="info" />;
    return <DocIcon {...iconProps} color="secondary" />;
  };

  const dimensions = {
    small: { width: 40, height: 40 },
    medium: { width: 80, height: 80 },
    large: { width: 120, height: 120 },
  };

  if (thumbnailUrl && !error) {
    return (
      <Box
        sx={{
          width: dimensions[size].width,
          height: dimensions[size].height,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <img
          src={thumbnailUrl}
          alt="Document thumbnail"
          style={{
            maxWidth: '100%',
            maxHeight: '100%',
            objectFit: 'cover',
            borderRadius: '4px',
          }}
        />
      </Box>
    );
  }

  if (fallbackIcon) {
    return (
      <Box
        sx={{
          width: dimensions[size].width,
          height: dimensions[size].height,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {getFileIcon(mimeType)}
      </Box>
    );
  }

  return null;
};

export default DocumentThumbnail;