import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  CircularProgress,
  Alert,
  Paper,
} from '@mui/material';
import { api } from '../services/api';

interface FailedDocumentViewerProps {
  failedDocumentId: string;
  filename: string;
  mimeType: string;
}

const FailedDocumentViewer: React.FC<FailedDocumentViewerProps> = ({
  failedDocumentId,
  filename,
  mimeType,
}) => {
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [documentUrl, setDocumentUrl] = useState<string | null>(null);

  useEffect(() => {
    loadFailedDocument();
    
    // Cleanup URL when component unmounts
    return () => {
      if (documentUrl) {
        window.URL.revokeObjectURL(documentUrl);
      }
    };
  }, [failedDocumentId]);

  const loadFailedDocument = async (): Promise<void> => {
    try {
      setLoading(true);
      setError(null);
      
      // Use the new failed document view endpoint
      const response = await api.get(`/documents/failed/${failedDocumentId}/view`, {
        responseType: 'blob'
      });
      
      const url = window.URL.createObjectURL(new Blob([response.data], { type: mimeType }));
      setDocumentUrl(url);
    } catch (err: any) {
      console.error('Failed to load failed document:', err);
      if (err.response?.status === 404) {
        setError('Document file not found or has been deleted');
      } else {
        setError('Failed to load document for viewing');
      }
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <Box sx={{ 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center', 
        minHeight: '200px' 
      }}>
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ m: 2 }}>
        <Typography variant="body2">{error}</Typography>
        <Typography variant="caption" sx={{ mt: 1, display: 'block' }}>
          The original file may have been deleted or moved from storage.
        </Typography>
      </Alert>
    );
  }

  return (
    <Paper elevation={2} sx={{ 
      p: 2, 
      borderRadius: 2,
      backgroundColor: 'background.paper',
      minHeight: '300px'
    }}>
      {documentUrl && (
        <>
          {mimeType.startsWith('image/') ? (
            <Box sx={{ textAlign: 'center' }}>
              <img
                src={documentUrl}
                alt={filename}
                style={{
                  maxWidth: '100%',
                  maxHeight: '400px',
                  objectFit: 'contain',
                }}
              />
            </Box>
          ) : mimeType === 'application/pdf' ? (
            <iframe
              src={documentUrl}
              width="100%"
              height="400px"
              style={{ border: 'none', borderRadius: '4px' }}
              title={filename}
            />
          ) : mimeType.startsWith('text/') ? (
            <Box sx={{ 
              fontFamily: 'monospace', 
              fontSize: '0.875rem',
              whiteSpace: 'pre-wrap',
              backgroundColor: 'grey.50',
              p: 2,
              borderRadius: 1,
              maxHeight: '400px',
              overflow: 'auto'
            }}>
              <iframe
                src={documentUrl}
                width="100%"
                height="400px"
                style={{ border: 'none' }}
                title={filename}
              />
            </Box>
          ) : (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Typography variant="body1" color="text.secondary">
                Cannot preview this file type ({mimeType})
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
                File: {filename}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                You can try downloading the file to view it locally.
              </Typography>
            </Box>
          )}
        </>
      )}
    </Paper>
  );
};

export default FailedDocumentViewer;