import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  CircularProgress,
  Alert,
  Paper,
} from '@mui/material';
import { documentService } from '../services/api';

interface DocumentViewerProps {
  documentId: string;
  filename: string;
  mimeType: string;
}

const DocumentViewer: React.FC<DocumentViewerProps> = ({
  documentId,
  filename,
  mimeType,
}) => {
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [documentUrl, setDocumentUrl] = useState<string | null>(null);

  useEffect(() => {
    loadDocument();
    
    // Cleanup URL when component unmounts
    return () => {
      if (documentUrl) {
        window.URL.revokeObjectURL(documentUrl);
      }
    };
  }, [documentId]);

  const loadDocument = async (): Promise<void> => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await documentService.view(documentId);
      const url = window.URL.createObjectURL(new Blob([response.data], { type: mimeType }));
      setDocumentUrl(url);
    } catch (err) {
      console.error('Failed to load document:', err);
      setError('Failed to load document for viewing');
    } finally {
      setLoading(false);
    }
  };

  const renderDocumentContent = (): React.ReactElement => {
    if (!documentUrl) return <></>;

    // Handle images
    if (mimeType.startsWith('image/')) {
      return (
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            minHeight: '60vh',
            p: 2,
          }}
        >
          <img
            src={documentUrl}
            alt={filename}
            style={{
              maxWidth: '100%',
              maxHeight: '100%',
              objectFit: 'contain',
              borderRadius: '8px',
              boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
            }}
          />
        </Box>
      );
    }

    // Handle PDFs
    if (mimeType === 'application/pdf') {
      return (
        <Box sx={{ height: '70vh', width: '100%' }}>
          <iframe
            src={documentUrl}
            width="100%"
            height="100%"
            style={{ border: 'none', borderRadius: '8px' }}
            title={filename}
          />
        </Box>
      );
    }

    // Handle text files
    if (mimeType.startsWith('text/')) {
      return (
        <TextFileViewer documentUrl={documentUrl} filename={filename} />
      );
    }

    // For other file types, show a message
    return (
      <Box sx={{ textAlign: 'center', py: 8 }}>
        <Typography variant="h6" color="text.secondary" sx={{ mb: 2 }}>
          Preview not available
        </Typography>
        <Typography variant="body2" color="text.secondary">
          This file type ({mimeType}) cannot be previewed in the browser.
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Please download the file to view its contents.
        </Typography>
      </Box>
    );
  };

  if (loading) {
    return (
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '60vh',
        }}
      >
        <CircularProgress sx={{ mb: 2 }} />
        <Typography variant="body2" color="text.secondary">
          Loading document...
        </Typography>
      </Box>
    );
  }

  if (error) {
    return (
      <Box sx={{ p: 3 }}>
        <Alert severity="error">{error}</Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', overflow: 'auto' }}>
      {renderDocumentContent()}
    </Box>
  );
};

// Component for viewing text files
const TextFileViewer: React.FC<{ documentUrl: string; filename: string }> = ({
  documentUrl,
  filename,
}) => {
  const [textContent, setTextContent] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    loadTextContent();
  }, [documentUrl]);

  const loadTextContent = async (): Promise<void> => {
    try {
      const response = await fetch(documentUrl);
      const text = await response.text();
      setTextContent(text);
    } catch (err) {
      console.error('Failed to load text content:', err);
      setTextContent('Failed to load text content');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
        <CircularProgress size={24} />
      </Box>
    );
  }

  return (
    <Paper
      sx={{
        p: 3,
        m: 2,
        backgroundColor: (theme) => theme.palette.mode === 'light' ? 'grey.50' : 'grey.900',
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 2,
        maxHeight: '70vh',
        overflow: 'auto',
      }}
    >
      <Typography
        variant="body2"
        sx={{
          fontFamily: 'monospace',
          whiteSpace: 'pre-wrap',
          lineHeight: 1.6,
          color: 'text.primary',
        }}
      >
        {textContent}
      </Typography>
    </Paper>
  );
};

export default DocumentViewer;